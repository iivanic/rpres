//! Minimal HTTP server built on `tiny_http`.

use std::io::Cursor;
use tiny_http::{Header, Response, Server};

/// Everything the server needs to answer requests.
pub struct App {
    /// The full HTML page served at `/`.
    pub page: String,
    /// Pre-rendered slide fragments served at `/slide/{n}` in paged mode.
    pub slides: Vec<String>,
    /// Whether paged mode is enabled.
    pub paged: bool,
}

type Resp = Response<Cursor<Vec<u8>>>;

fn html_response(body: String) -> Resp {
    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
        .expect("valid header");
    Response::from_string(body).with_header(header)
}

fn not_found() -> Resp {
    Response::from_string("Not found").with_status_code(404)
}

/// Route a request URL to a response.
pub fn route(app: &App, url: &str) -> Resp {
    let path = url.split('?').next().unwrap_or(url);
    if path == "/" || path == "/index.html" {
        return html_response(app.page.clone());
    }
    if app.paged {
        if let Some(rest) = path.strip_prefix("/slide/") {
            if let Ok(n) = rest.parse::<usize>() {
                if let Some(slide) = app.slides.get(n) {
                    return html_response(slide.clone());
                }
            }
        }
    }
    not_found()
}

/// Bind to `addr` and serve requests until the process is stopped.
pub fn serve(addr: &str, app: App) -> std::io::Result<()> {
    let server = Server::http(addr)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    for request in server.incoming_requests() {
        let response = route(&app, request.url());
        request.respond(response)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App {
            page: "<html>page</html>".to_string(),
            slides: vec!["<section>0</section>".into(), "<section>1</section>".into()],
            paged: true,
        }
    }

    fn status(resp: &Resp) -> u16 {
        resp.status_code().0
    }

    #[test]
    fn serves_index() {
        assert_eq!(status(&route(&app(), "/")), 200);
    }

    #[test]
    fn serves_slide_by_index() {
        assert_eq!(status(&route(&app(), "/slide/1")), 200);
    }

    #[test]
    fn missing_slide_is_404() {
        assert_eq!(status(&route(&app(), "/slide/9")), 404);
    }

    #[test]
    fn slide_route_disabled_when_not_paged() {
        let mut a = app();
        a.paged = false;
        assert_eq!(status(&route(&a, "/slide/0")), 404);
    }
}
