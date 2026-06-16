//! Render slides to HTML and assemble the full presentation page.

use crate::slides::Slide;
use crate::templates::{BASE_CSS, JS, SKELETON, theme_css};
use pulldown_cmark::{Event, Options, Parser, html};

fn options() -> Options {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);
    opts
}

/// Convert a Markdown fragment to an HTML string.
pub fn md_to_html(md: &str) -> String {
    let parser = Parser::new_ext(md, options());
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

/// Split a Markdown body into top-level block HTML fragments.
fn body_blocks(md: &str) -> Vec<String> {
    let parser = Parser::new_ext(md, options());
    let mut blocks = Vec::new();
    let mut buffer: Vec<Event> = Vec::new();
    let mut depth: i32 = 0;

    for event in parser {
        match &event {
            Event::Start(_) => depth += 1,
            Event::End(_) => depth -= 1,
            _ => {}
        }
        buffer.push(event);
        if depth == 0 {
            flush_block(&mut buffer, &mut blocks);
        }
    }
    flush_block(&mut buffer, &mut blocks);
    blocks
}

fn flush_block(buffer: &mut Vec<Event>, blocks: &mut Vec<String>) {
    if buffer.is_empty() {
        return;
    }
    let mut html_out = String::new();
    html::push_html(&mut html_out, buffer.drain(..));
    if !html_out.trim().is_empty() {
        blocks.push(html_out.trim().to_string());
    }
}

/// Render a single slide to its HTML `<section>`.
pub fn slide_html(slide: &Slide) -> String {
    let mut lines = slide.raw.lines();
    let header_html;
    let body_md;

    if slide.has_header() {
        let first = lines.next().unwrap_or_default();
        header_html = md_to_html(first);
        body_md = lines.collect::<Vec<_>>().join("\n");
    } else {
        header_html = String::new();
        body_md = slide.raw.clone();
    }

    let mut fragments = String::new();
    for block in body_blocks(&body_md) {
        if is_list_block(&block) {
            // Lists reveal one item at a time in animation mode, so each
            // `<li>` becomes its own fragment instead of the whole list.
            fragments.push_str(&mark_list_items(&block));
            fragments.push('\n');
        } else {
            fragments.push_str("<div class=\"fragment\">");
            fragments.push_str(&block);
            fragments.push_str("</div>\n");
        }
    }

    let class = if slide.is_title() {
        "slide title"
    } else {
        "slide"
    };

    format!(
        "<section class=\"{class}\">\n<div class=\"slide-head\">{}</div>\n<div class=\"slide-body\">\n{fragments}</div>\n</section>\n",
        header_html.trim()
    )
}

/// Whether an HTML block is an ordered or unordered list.
fn is_list_block(block: &str) -> bool {
    let t = block.trim_start();
    t.starts_with("<ul") || t.starts_with("<ol")
}

/// Add the `fragment` class to every `<li>` in a list block so that list
/// items can be revealed one at a time in animation mode.
fn mark_list_items(block: &str) -> String {
    block
        .replace("<li class=\"", "<li class=\"fragment ")
        .replace("<li>", "<li class=\"fragment\">")
}

/// Render every slide and join the results.
pub fn render_deck(slides: &[Slide]) -> String {
    slides.iter().map(slide_html).collect()
}

/// Build the full HTML page.
///
/// In default (non-paged) mode the deck is filled with all slides. In paged
/// mode the deck starts empty and slides are fetched over AJAX.
pub fn build_page(
    title: &str,
    template: &str,
    slides: &[Slide],
    anim: bool,
    paged: bool,
) -> String {
    let theme = theme_css(template).unwrap_or_else(|| theme_css("modern").unwrap());
    let deck = if paged { String::new() } else { render_deck(slides) };

    SKELETON
        .replace("{{TITLE}}", &escape_html(title))
        .replace("{{BASE_CSS}}", BASE_CSS)
        .replace("{{THEME_CSS}}", theme)
        .replace("{{ANIM}}", if anim { "1" } else { "0" })
        .replace("{{PAGED}}", if paged { "1" } else { "0" })
        .replace("{{COUNT}}", &slides.len().to_string())
        .replace("{{DECK}}", &deck)
        .replace("{{JS}}", JS)
}

/// Minimal HTML escaping for attribute / text contexts.
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slides::parse_slides;

    #[test]
    fn renders_basic_markdown() {
        let html = md_to_html("**bold** and *italic*");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn renders_tables() {
        let html = md_to_html("| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>1</td>"));
    }

    #[test]
    fn body_split_into_blocks() {
        let blocks = body_blocks("first paragraph\n\nsecond paragraph\n\n- a\n- b\n");
        assert_eq!(blocks.len(), 3);
        assert!(blocks[2].contains("<ul>"));
    }

    #[test]
    fn slide_wraps_blocks_in_fragments() {
        let slides = parse_slides("## Slide\nline one\n\nline two\n");
        let html = slide_html(&slides[0]);
        assert_eq!(html.matches("class=\"fragment\"").count(), 2);
        assert!(html.contains("<h2>Slide</h2>"));
    }

    #[test]
    fn title_slide_gets_title_class() {
        let slides = parse_slides("# Hello\nsub\n");
        let html = slide_html(&slides[0]);
        assert!(html.contains("class=\"slide title\""));
    }

    #[test]
    fn list_items_become_individual_fragments() {
        let slides = parse_slides("## Slide\n- one\n- two\n- three\n");
        let html = slide_html(&slides[0]);
        // Each <li> is a fragment; the <ul> itself is not wrapped.
        assert_eq!(html.matches("<li class=\"fragment\">").count(), 3);
        assert!(!html.contains("<div class=\"fragment\"><ul>"));
    }

    #[test]
    fn task_list_items_become_fragments() {
        let slides = parse_slides("## Slide\n- [x] done\n- [ ] todo\n");
        let html = slide_html(&slides[0]);
        assert_eq!(html.matches("<li class=\"fragment\">").count(), 2);
        assert!(html.contains("type=\"checkbox\""));
    }

    #[test]
    fn page_embeds_slides_in_default_mode() {
        let slides = parse_slides("# T\n\n## One\nx\n");
        let page = build_page("T", "modern", &slides, false, false);
        assert!(page.contains("data-count=\"2\""));
        assert!(page.contains("data-paged=\"0\""));
        assert!(page.contains("<section class=\"slide title\">"));
    }

    #[test]
    fn page_is_empty_in_paged_mode() {
        let slides = parse_slides("# T\n\n## One\nx\n");
        let page = build_page("T", "terminal", &slides, false, true);
        assert!(page.contains("data-paged=\"1\""));
        assert!(page.contains("<div id=\"deck\"></div>"));
    }

    #[test]
    fn unknown_template_falls_back_to_modern() {
        let slides = parse_slides("# T\n");
        let page = build_page("T", "nope", &slides, false, false);
        // modern theme uses a gradient background
        assert!(page.contains("radial-gradient"));
    }
}
