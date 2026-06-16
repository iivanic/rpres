//! End-to-end tests exercising the public API on the demo presentation.

use rpres::render::{build_page, slide_html};
use rpres::server::{App, route};
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
    let page = build_page("Demo", "modern", &slides, false, false);
    let count = page.matches("<section class=\"slide").count();
    assert_eq!(count, slides.len());
}

#[test]
fn paged_server_serves_each_slide() {
    let slides = parse_slides(DEMO);
    let page = build_page("Demo", "terminal", &slides, false, true);
    let app = App {
        page,
        slides: slides.iter().map(slide_html).collect(),
        paged: true,
    };

    assert_eq!(route(&app, "/").status_code().0, 200);
    for i in 0..slides.len() {
        let url = format!("/slide/{i}");
        assert_eq!(route(&app, &url).status_code().0, 200, "slide {i} missing");
    }
    assert_eq!(route(&app, "/slide/9999").status_code().0, 404);
}

#[test]
fn every_template_renders() {
    let slides = parse_slides(DEMO);
    for template in ["terminal", "classic", "modern"] {
        let page = build_page("Demo", template, &slides, true, false);
        assert!(page.contains("<!DOCTYPE html>"));
        assert!(page.contains("data-anim=\"1\""));
    }
}
