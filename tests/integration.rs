//! End-to-end tests exercising the public API on the demo presentation.

use rpres::render::build_page;
use rpres::slides::parse_slides;

const DEMO: &str = include_str!("../demo.md");

#[test]
fn demo_parses_into_multiple_slides() {
    let slides = parse_slides(DEMO);
    assert!(slides.len() >= 8, "expected many slides, got {}", slides.len());
    assert!(slides[0].is_title(), "first slide should be a title slide");
}

#[test]
fn demo_code_fence_does_not_split_slides() {
    let slides = parse_slides(DEMO);
    let code_slide = slides
        .iter()
        .find(|s| s.title == "Code Blocks")
        .expect("Code Blocks slide present");
    assert!(code_slide.raw.contains("# this hash should not start a new slide"));
}

#[test]
fn default_page_embeds_all_slides() {
    let slides = parse_slides(DEMO);
    let page = build_page("Demo", "modern", &slides, false, "v1");
    let count = page.matches("<section class=\"slide").count();
    assert_eq!(count, slides.len());
}

#[test]
fn every_template_renders() {
    let slides = parse_slides(DEMO);
    for template in ["terminal", "classic", "modern"] {
        let page = build_page("Demo", template, &slides, true, "v1");
        assert!(page.contains("<!DOCTYPE html>"));
        assert!(page.contains("data-anim=\"1\""));
    }
}

#[test]
fn page_includes_print_stylesheet() {
    let slides = parse_slides(DEMO);
    let page = build_page("Demo", "modern", &slides, false, "v1");
    // Print mode relies on @media print page-break rules being embedded.
    assert!(page.contains("@media print"));
    assert!(page.contains("break-after: page"));
}

#[test]
fn print_mode_embeds_every_slide() {
    let slides = parse_slides(DEMO);
    // Printing relies on the full deck being in the document.
    let page = build_page("Demo", "modern", &slides, false, "v1");
    let count = page.matches("<section class=\"slide").count();
    assert_eq!(count, slides.len());
}
