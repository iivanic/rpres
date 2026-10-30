//! Minimal HTTP server built on `tiny_http`.
//!
//! The deck is rendered on demand and re-rendered whenever the source Markdown
//! file changes, so an open browser tab can pick the changes up (hot reload).
//! Files next to the Markdown source (images, fonts, …) are served as-is.

use crate::render::{build_page, deck_title};
use crate::slides::parse_slides;
use std::fs;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;
use tiny_http::{Header, Response, Server};

/// Modification time and size of the Markdown source; any change means the
/// deck has to be re-rendered.
type Fingerprint = (Option<SystemTime>, u64);

/// A presentation served over HTTP.
pub struct Deck {
    source: PathBuf,
    base_dir: PathBuf,
    template: String,
    anim: bool,
    state: Mutex<State>,
}

struct State {
    page: String,
    version: String,
    count: usize,
    fingerprint: Option<Fingerprint>,
    last_error: Option<String>,
}

type Resp = Response<Cursor<Vec<u8>>>;

impl Deck {
    /// Read and render `source` once; returns an error message on failure.
    pub fn new(source: &Path, template: &str, anim: bool) -> Result<Deck, String> {
        let (page, version, count, fingerprint) = render(source, template, anim)?;
        let base_dir = match source.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => dir.to_path_buf(),
            _ => PathBuf::from("."),
        };
        Ok(Deck {
            source: source.to_path_buf(),
            base_dir,
            template: template.to_string(),
            anim,
            state: Mutex::new(State {
                page,
                version,
                count,
                fingerprint,
                last_error: None,
            }),
        })
    }

    /// Number of slides in the version that was rendered last.
    pub fn slide_count(&self) -> usize {
        self.lock().count
    }

    /// The current page HTML, re-rendering first if the source changed.
    fn page(&self) -> String {
        let mut state = self.lock();
        self.refresh(&mut state);
        state.page.clone()
    }

    /// Identifier of the currently rendered source, used by hot reload.
    fn version(&self) -> String {
        let mut state = self.lock();
        self.refresh(&mut state);
        state.version.clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        // A panic while rendering must not take the whole server down.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Re-render when the source file changed, keeping the last good page if
    /// the new one cannot be rendered (e.g. a half-written save).
    fn refresh(&self, state: &mut State) {
        let fingerprint = fingerprint(&self.source);
        if fingerprint.is_some() && fingerprint == state.fingerprint {
            return;
        }
        match render(&self.source, &self.template, self.anim) {
            Ok((page, version, count, fingerprint)) => {
                state.page = page;
                state.version = version;
                state.count = count;
                state.fingerprint = fingerprint;
                state.last_error = None;
            }
            Err(err) => {
                if state.last_error.as_deref() != Some(err.as_str()) {
                    eprintln!("warning: {err} (still serving the previous version)");
                    state.last_error = Some(err);
                }
            }
        }
    }
}

/// Read, parse and render the source file.
fn render(
    source: &Path,
    template: &str,
    anim: bool,
) -> Result<(String, String, usize, Option<Fingerprint>), String> {
    // Fingerprint before reading: if the file changes mid-render, the stored
    // version is older than the content, so the browser re-renders once more
    // instead of getting stuck on a stale page.
    let fingerprint = fingerprint(source);
    let markdown = fs::read_to_string(source)
        .map_err(|e| format!("could not read '{}': {e}", source.display()))?;
    let slides = parse_slides(&markdown);
    if slides.is_empty() {
        return Err(format!("'{}' contains no slides", source.display()));
    }
    let version = version_of(fingerprint);
    let page = build_page(
        &deck_title(&slides, source),
        template,
        &slides,
        anim,
        &version,
    );
    Ok((page, version, slides.len(), fingerprint))
}

fn fingerprint(path: &Path) -> Option<Fingerprint> {
    let meta = fs::metadata(path).ok()?;
    Some((meta.modified().ok(), meta.len()))
}

fn version_of(fingerprint: Option<Fingerprint>) -> String {
    let (modified, len) = fingerprint.unwrap_or((None, 0));
    let nanos = modified
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos}-{len}")
}

fn html_response(body: String) -> Resp {
    Response::from_string(body)
        .with_header(content_type_header("text/html; charset=utf-8"))
        .with_header(no_store())
}

fn text_response(body: String) -> Resp {
    Response::from_string(body)
        .with_header(content_type_header("text/plain; charset=utf-8"))
        .with_header(no_store())
}

fn not_found() -> Resp {
    Response::from_string("Not found").with_status_code(404)
}

fn content_type_header(value: &str) -> Header {
    Header::from_bytes(&b"Content-Type"[..], value.as_bytes()).expect("valid header")
}

fn no_store() -> Header {
    Header::from_bytes(&b"Cache-Control"[..], &b"no-store"[..]).expect("valid header")
}

/// Route a request URL to a response.
pub fn route(deck: &Deck, url: &str) -> Resp {
    let path = url.split('?').next().unwrap_or(url);
    match path {
        "/" | "/index.html" => html_response(deck.page()),
        "/version" => text_response(deck.version()),
        _ => static_file(deck, path).unwrap_or_else(not_found),
    }
}

/// Serve a file from the Markdown source's directory.
fn static_file(deck: &Deck, path: &str) -> Option<Resp> {
    let relative = percent_decode(path.trim_start_matches('/'));
    let mut target = deck.base_dir.clone();
    for component in Path::new(&relative).components() {
        match component {
            Component::Normal(name) => target.push(name),
            Component::CurDir => {}
            // Never let a request escape the directory being served.
            _ => return None,
        }
    }
    if target == deck.base_dir {
        return None;
    }
    let data = fs::read(&target).ok()?;
    Some(
        Response::from_data(data)
            .with_header(content_type_header(content_type(&target)))
            .with_header(no_store()),
    )
}

fn content_type(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(high), Some(low)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2]))
        {
            out.push(high * 16 + low);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Bind to `addr` and serve requests until the process is stopped.
pub fn serve(addr: &str, deck: Deck) -> std::io::Result<()> {
    let server = Server::http(addr).map_err(|e| std::io::Error::other(e.to_string()))?;
    for request in server.incoming_requests() {
        let response = route(&deck, request.url());
        if let Err(err) = request.respond(response) {
            eprintln!("warning: failed to send response: {err}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    /// A throw-away directory holding a deck plus one image.
    struct Fixture {
        dir: PathBuf,
        markdown: PathBuf,
    }

    impl Fixture {
        fn new(source: &str) -> Fixture {
            let dir = std::env::temp_dir().join(format!(
                "rpres-test-{}-{}",
                std::process::id(),
                NEXT_DIR.fetch_add(1, Ordering::SeqCst)
            ));
            fs::create_dir_all(&dir).unwrap();
            let markdown = dir.join("deck.md");
            fs::write(&markdown, source).unwrap();
            fs::write(dir.join("px.png"), b"\x89PNG\r\n\x1a\n").unwrap();
            Fixture { dir, markdown }
        }

        fn deck(&self) -> Deck {
            Deck::new(&self.markdown, "modern", false).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn status(resp: &Resp) -> u16 {
        resp.status_code().0
    }

    fn body(resp: Resp) -> String {
        String::from_utf8(bytes(resp)).unwrap()
    }

    fn bytes(resp: Resp) -> Vec<u8> {
        let mut out = Vec::new();
        resp.into_reader().read_to_end(&mut out).unwrap();
        out
    }

    const DECK: &str = "# Title\n\n## One\nfirst\n";

    #[test]
    fn serves_index() {
        let fixture = Fixture::new(DECK);
        let page = body(route(&fixture.deck(), "/"));
        assert!(page.contains("<section class=\"slide title\">"));
        assert!(page.contains("data-count=\"2\""));
    }

    #[test]
    fn serves_local_images_next_to_the_deck() {
        let fixture = Fixture::new(DECK);
        let resp = route(&fixture.deck(), "/px.png");
        assert_eq!(status(&resp), 200);
        assert_eq!(bytes(resp), b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn decodes_percent_encoded_paths() {
        let fixture = Fixture::new(DECK);
        fs::write(fixture.dir.join("my image.png"), b"data").unwrap();
        assert_eq!(status(&route(&fixture.deck(), "/my%20image.png")), 200);
    }

    #[test]
    fn refuses_to_escape_the_deck_directory() {
        let fixture = Fixture::new(DECK);
        assert_eq!(status(&route(&fixture.deck(), "/../deck.md")), 404);
        assert_eq!(status(&route(&fixture.deck(), "/%2e%2e/deck.md")), 404);
    }

    #[test]
    fn missing_files_are_404() {
        let fixture = Fixture::new(DECK);
        assert_eq!(status(&route(&fixture.deck(), "/nope.png")), 404);
    }

    #[test]
    fn version_is_stable_until_the_source_changes() {
        let fixture = Fixture::new(DECK);
        let deck = fixture.deck();
        let before = body(route(&deck, "/version"));
        assert!(!before.is_empty());
        assert_eq!(before, body(route(&deck, "/version")));

        fs::write(&fixture.markdown, "# Title\n\n## Two\nsecond\n").unwrap();
        let after = body(route(&deck, "/version"));
        assert_ne!(before, after, "hot reload version must change");
        assert!(body(route(&deck, "/")).contains("second"));
    }

    #[test]
    fn keeps_serving_the_last_good_page_while_the_source_is_broken() {
        let fixture = Fixture::new(DECK);
        let deck = fixture.deck();
        let before = body(route(&deck, "/version"));

        fs::write(&fixture.markdown, "").unwrap();
        assert!(body(route(&deck, "/")).contains("first"));
        assert_eq!(before, body(route(&deck, "/version")));
    }

    #[test]
    fn a_deck_without_slides_is_rejected() {
        let fixture = Fixture::new("   \n\n");
        assert!(Deck::new(&fixture.markdown, "modern", false).is_err());
    }
}
