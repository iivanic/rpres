//! Split a Markdown document into presentation slides.
//!
//! Slides are delimited by headers:
//! * a level-1 header (`# ...`) starts a *title* slide,
//! * a level-2 header (`## ...`) starts a normal slide.
//!
//! Headers of level 3 and deeper, as well as `#` characters inside fenced
//! code blocks, are treated as ordinary slide content.

/// A single slide of the presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slide {
    /// Raw Markdown source of the slide, including its header line.
    pub raw: String,
    /// Header level: `Some(1)` for a title slide, `Some(2)` for a normal
    /// slide, `None` for content that appears before the first header.
    pub level: Option<usize>,
    /// Plain-text header, with the leading `#` characters stripped.
    pub title: String,
}

impl Slide {
    /// Whether this slide is a title (level-1) slide.
    pub fn is_title(&self) -> bool {
        self.level == Some(1)
    }

    /// Whether this slide starts with a header line.
    pub fn has_header(&self) -> bool {
        self.level.is_some()
    }
}

/// Returns the slide header level (1 or 2) for `line`, or `None` if the line
/// is not a slide-delimiting header.
pub fn header_level(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    let rest = &trimmed[hashes..];
    if (1..=2).contains(&hashes) && (rest.is_empty() || rest.starts_with(' ')) {
        Some(hashes)
    } else {
        None
    }
}

/// Strip the leading `#` characters and surrounding whitespace from a header.
fn header_text(line: &str) -> String {
    line.trim_start()
        .trim_start_matches('#')
        .trim()
        .to_string()
}

/// Parse a Markdown document into a list of [`Slide`]s.
pub fn parse_slides(md: &str) -> Vec<Slide> {
    let mut slides = Vec::new();
    let mut current: Option<(String, Option<usize>)> = None;
    let mut in_fence = false;
    let mut fence_marker = '`';

    for line in md.lines() {
        let trimmed = line.trim_start();

        // Toggle fenced-code-block state.
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let marker = trimmed.chars().next().unwrap();
            if !in_fence {
                in_fence = true;
                fence_marker = marker;
            } else if marker == fence_marker {
                in_fence = false;
            }
        }

        let level = if in_fence { None } else { header_level(line) };

        if let Some(level) = level {
            if let Some((raw, lvl)) = current.take() {
                slides.push(make_slide(raw, lvl));
            }
            current = Some((String::new(), Some(level)));
        } else if current.is_none() {
            // Content before the first header becomes an untitled slide.
            current = Some((String::new(), None));
        }

        if let Some((raw, _)) = current.as_mut() {
            raw.push_str(line);
            raw.push('\n');
        }
    }

    if let Some((raw, lvl)) = current.take() {
        slides.push(make_slide(raw, lvl));
    }

    slides
        .into_iter()
        .filter(|s| !s.raw.trim().is_empty())
        .collect()
}

fn make_slide(raw: String, level: Option<usize>) -> Slide {
    let title = match level {
        Some(_) => raw.lines().next().map(header_text).unwrap_or_default(),
        None => String::new(),
    };
    Slide { raw, level, title }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_header_levels() {
        assert_eq!(header_level("# Title"), Some(1));
        assert_eq!(header_level("## Slide"), Some(2));
        assert_eq!(header_level("### Sub"), None);
        assert_eq!(header_level("Not a header"), None);
        assert_eq!(header_level("#NoSpace"), None);
        assert_eq!(header_level("  ## Indented"), Some(2));
    }

    #[test]
    fn splits_on_headers() {
        let md = "# Welcome\nintro\n\n## One\nbody one\n\n## Two\nbody two\n";
        let slides = parse_slides(md);
        assert_eq!(slides.len(), 3);
        assert!(slides[0].is_title());
        assert_eq!(slides[0].title, "Welcome");
        assert_eq!(slides[1].title, "One");
        assert!(!slides[1].is_title());
        assert_eq!(slides[2].title, "Two");
    }

    #[test]
    fn ignores_headers_inside_code_fences() {
        let md = "## Slide\n```sh\n# this is a shell comment\necho hi\n```\n";
        let slides = parse_slides(md);
        assert_eq!(slides.len(), 1);
        assert!(slides[0].raw.contains("# this is a shell comment"));
    }

    #[test]
    fn keeps_deep_headers_in_slide() {
        let md = "## Slide\n### Subsection\ncontent\n";
        let slides = parse_slides(md);
        assert_eq!(slides.len(), 1);
        assert!(slides[0].raw.contains("### Subsection"));
    }

    #[test]
    fn preamble_becomes_untitled_slide() {
        let md = "loose text\n\n# Title\nx\n";
        let slides = parse_slides(md);
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0].level, None);
        assert!(slides[1].is_title());
    }
}
