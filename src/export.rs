//! Export a presentation to PDF — one slide per page, with embedded fonts and
//! images.
//!
//! The exporter uses the pure-Rust [`printpdf`] crate (MIT licensed). Fonts are
//! always embedded (a Unicode TrueType font is located on the system or supplied
//! with `--font`), images referenced by local path are embedded, and the
//! document is tagged with PDF/A-1b conformance metadata so it is suitable for
//! archiving. Each slide becomes exactly one page.

use crate::slides::Slide;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::path::{Path, PathBuf};

use printpdf::{
    Color, FontId, Mm, Op, ParsedFont, PdfConformance, PdfDocument, PdfFontHandle, PdfPage,
    PdfSaveOptions, Point, Pt, RawImage, Rgb, TextItem, XObjectTransform,
};

/// Points per millimetre (72 / 25.4).
const MM_TO_PT: f32 = 2.834_645_7;

/// Landscape 16:9-ish page, in millimetres.
const PAGE_W_MM: f32 = 297.0;
const PAGE_H_MM: f32 = 167.0;
const MARGIN_MM: f32 = 18.0;

/// Average glyph-width factor (fraction of font size) used for line wrapping.
const REGULAR_FACTOR: f32 = 0.52;
const MONO_FACTOR: f32 = 0.6;

/// Options controlling a PDF export.
pub struct ExportOptions {
    /// Document title stored in the PDF metadata.
    pub title: String,
    /// Tag the document as PDF/A-1b (embedded fonts, archival metadata).
    pub pdfa: bool,
    /// Directory used to resolve relative image paths (the Markdown file's dir).
    pub base_dir: PathBuf,
    /// Optional path to a TrueType font to embed instead of a system font.
    pub font_path: Option<PathBuf>,
}

/// A laid-out block of slide content.
enum Block {
    Heading { level: u8, text: String },
    Para(String),
    Bullet { depth: u8, marker: String, text: String },
    Code(Vec<String>),
    Quote(String),
    Rule,
    Image { src: String, alt: String },
    TableRow { text: String, header: bool },
}

/// Render the slides to PDF bytes.
pub fn export_pdf(slides: &[Slide], opts: &ExportOptions) -> Result<Vec<u8>, String> {
    let regular_bytes = load_regular_font(opts.font_path.as_deref())
        .ok_or_else(|| no_font_message())?;
    let mono_bytes = load_mono_font();

    let mut warnings = Vec::new();
    let regular = ParsedFont::from_bytes(&regular_bytes, 0, &mut warnings)
        .ok_or_else(|| "failed to parse the regular font".to_string())?;
    let mono_parsed = mono_bytes
        .as_ref()
        .and_then(|b| ParsedFont::from_bytes(b, 0, &mut warnings));

    let mut doc = PdfDocument::new(&opts.title);
    doc.metadata.info.document_title = opts.title.clone();
    doc.metadata.info.creator = "rpres".to_string();
    doc.metadata.info.producer = "rpres (printpdf)".to_string();
    if opts.pdfa {
        doc.metadata.info.conformance = PdfConformance::A1B_2005_PDF_1_4;
    }

    let regular_id = doc.add_font(&regular);
    let mono_id = mono_parsed.as_ref().map(|m| doc.add_font(m));

    let regular_font = Fonts {
        text: regular_id.clone(),
        mono: mono_id.clone().unwrap_or(regular_id),
        has_mono: mono_id.is_some(),
    };

    let mut pages = Vec::with_capacity(slides.len());
    for slide in slides {
        let blocks = parse_blocks(&slide.raw);
        let ops = layout_page(&mut doc, &regular_font, &blocks, slide.is_title(), opts);
        pages.push(PdfPage::new(Mm(PAGE_W_MM), Mm(PAGE_H_MM), ops));
    }

    doc.with_pages(pages);
    let mut save_warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut save_warnings);
    Ok(bytes)
}

/// Render the slides and write the resulting PDF to `path`.
pub fn write_pdf_file(
    slides: &[Slide],
    path: &Path,
    opts: &ExportOptions,
) -> Result<(), String> {
    let bytes = export_pdf(slides, opts)?;
    std::fs::write(path, bytes).map_err(|e| format!("could not write '{}': {e}", path.display()))
}

/// Font handles resolved on the document.
struct Fonts {
    text: FontId,
    mono: FontId,
    has_mono: bool,
}

/// Lay out one slide onto a list of page operations.
fn layout_page(
    doc: &mut PdfDocument,
    fonts: &Fonts,
    blocks: &[Block],
    is_title: bool,
    opts: &ExportOptions,
) -> Vec<Op> {
    let mut ops = Vec::new();
    let content_w = (PAGE_W_MM - 2.0 * MARGIN_MM) * MM_TO_PT;
    let left = MARGIN_MM * MM_TO_PT;
    let top = (PAGE_H_MM - MARGIN_MM) * MM_TO_PT;
    let bottom = MARGIN_MM * MM_TO_PT;
    let mut y = top;

    let heading_color = Color::Rgb(Rgb::new(0.12, 0.34, 0.66, None));
    let body_color = Color::Rgb(Rgb::new(0.1, 0.1, 0.12, None));
    let quote_color = Color::Rgb(Rgb::new(0.35, 0.35, 0.4, None));
    let code_color = Color::Rgb(Rgb::new(0.15, 0.15, 0.18, None));

    for block in blocks {
        if y <= bottom {
            break; // one page per slide; silently clip overflow
        }
        match block {
            Block::Heading { level, text } => {
                let size = match level {
                    1 => 30.0,
                    2 => 22.0,
                    _ => 17.0,
                };
                let lines = wrap(text, size, REGULAR_FACTOR, content_w);
                for line in lines {
                    let x = if is_title && *level == 1 {
                        let w = est_width(&line, size, REGULAR_FACTOR);
                        left + (content_w - w).max(0.0) / 2.0
                    } else {
                        left
                    };
                    draw_line(&mut ops, &fonts.text, size, &heading_color, x, y, &line);
                    y -= size * 1.35;
                }
                y -= size * 0.35;
            }
            Block::Para(text) => {
                y = draw_paragraph(
                    &mut ops, &fonts.text, 13.0, REGULAR_FACTOR, &body_color, left,
                    content_w, y, text,
                );
                y -= 6.0;
            }
            Block::Bullet { depth, marker, text } => {
                let size = 13.0;
                let indent = left + (*depth as f32) * 16.0;
                let marker_w = est_width(marker, size, REGULAR_FACTOR) + 4.0;
                let avail = content_w - (indent - left) - marker_w;
                let lines = wrap(text, size, REGULAR_FACTOR, avail.max(40.0));
                for (i, line) in lines.iter().enumerate() {
                    if i == 0 {
                        draw_line(&mut ops, &fonts.text, size, &body_color, indent, y, marker);
                    }
                    draw_line(
                        &mut ops, &fonts.text, size, &body_color, indent + marker_w, y, line,
                    );
                    y -= size * 1.3;
                }
            }
            Block::Code(code_lines) => {
                let size = 11.0;
                let font = if fonts.has_mono { &fonts.mono } else { &fonts.text };
                let factor = if fonts.has_mono { MONO_FACTOR } else { REGULAR_FACTOR };
                let max_chars = ((content_w / (size * factor)).floor() as usize).max(8);
                y -= 2.0;
                for raw in code_lines {
                    for line in hard_wrap(raw, max_chars) {
                        draw_line(&mut ops, font, size, &code_color, left + 6.0, y, &line);
                        y -= size * 1.3;
                    }
                }
                y -= 6.0;
            }
            Block::Quote(text) => {
                let size = 13.0;
                let lines = wrap(text, size, REGULAR_FACTOR, content_w - 16.0);
                for line in lines {
                    draw_line(
                        &mut ops, &fonts.text, size, &quote_color, left + 14.0, y,
                        &format!("\u{201c}{line}\u{201d}"),
                    );
                    y -= size * 1.35;
                }
                y -= 6.0;
            }
            Block::Rule => {
                y -= 10.0;
            }
            Block::Image { src, alt } => {
                match embed_image(doc, &mut ops, src, opts, left, content_w, y) {
                    Some(used_h) => {
                        y -= used_h + 8.0;
                    }
                    None => {
                        let note = format!("[image: {}]", if alt.is_empty() { src } else { alt });
                        draw_line(&mut ops, &fonts.text, 12.0, &quote_color, left, y, &note);
                        y -= 18.0;
                    }
                }
            }
            Block::TableRow { text, header } => {
                let size = 11.5;
                let font = if fonts.has_mono { &fonts.mono } else { &fonts.text };
                let color = if *header { &heading_color } else { &body_color };
                draw_line(&mut ops, font, size, color, left, y, text);
                y -= size * 1.4;
            }
        }
    }

    ops
}

/// Draw a paragraph with word wrapping; returns the updated `y`.
#[allow(clippy::too_many_arguments)]
fn draw_paragraph(
    ops: &mut Vec<Op>,
    font: &FontId,
    size: f32,
    factor: f32,
    color: &Color,
    x: f32,
    max_w: f32,
    mut y: f32,
    text: &str,
) -> f32 {
    for line in wrap(text, size, factor, max_w) {
        draw_line(ops, font, size, color, x, y, &line);
        y -= size * 1.35;
    }
    y
}

/// Emit the operations to draw a single line of text at an absolute position.
///
/// Each line is wrapped in its own `BT`/`ET` text section so that the
/// `SetTextCursor` (a relative `Td` operation) is measured from the page
/// origin, giving absolute placement.
fn draw_line(ops: &mut Vec<Op>, font: &FontId, size: f32, color: &Color, x: f32, y: f32, text: &str) {
    if text.is_empty() {
        return;
    }
    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font: PdfFontHandle::External(font.clone()),
        size: Pt(size),
    });
    ops.push(Op::SetFillColor { col: color.clone() });
    ops.push(Op::SetTextCursor {
        pos: Point { x: Pt(x), y: Pt(y) },
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(text.to_string())],
    });
    ops.push(Op::EndTextSection);
}

/// Decode and place a local image; returns the height it consumed, or `None`
/// if the image could not be embedded (e.g. remote URL or unreadable file).
fn embed_image(
    doc: &mut PdfDocument,
    ops: &mut Vec<Op>,
    src: &str,
    opts: &ExportOptions,
    left: f32,
    content_w: f32,
    y: f32,
) -> Option<f32> {
    if src.starts_with("http://") || src.starts_with("https://") || src.starts_with("data:") {
        return None; // only local files are embedded
    }
    let path = opts.base_dir.join(src);
    let bytes = std::fs::read(&path).ok()?;
    let mut warnings = Vec::new();
    let image = RawImage::decode_from_bytes(&bytes, &mut warnings).ok()?;

    let natural_w = image.width as f32 / 300.0 * 72.0;
    let natural_h = image.height as f32 / 300.0 * 72.0;
    if natural_w <= 0.0 || natural_h <= 0.0 {
        return None;
    }
    let target_w = content_w.min(natural_w);
    let scale = target_w / natural_w;
    let target_h = natural_h * scale;

    let id = doc.add_image(&image);
    ops.push(Op::UseXobject {
        id,
        transform: XObjectTransform {
            translate_x: Some(Pt(left)),
            translate_y: Some(Pt(y - target_h)),
            rotate: None,
            scale_x: Some(scale),
            scale_y: Some(scale),
            dpi: Some(300.0),
        },
    });
    Some(target_h)
}

/// Parse a slide's Markdown into a flat list of [`Block`]s for layout.
fn parse_blocks(md: &str) -> Vec<Block> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let mut blocks = Vec::new();
    let mut text = String::new();
    let mut heading_level: Option<u8> = None;
    let mut in_code = false;
    let mut code_lines: Vec<String> = Vec::new();
    let mut in_quote = false;
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut pending_image: Option<String> = None;
    let mut table_row: Vec<String> = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();

    for event in Parser::new_ext(md, opts) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                text.clear();
                heading_level = Some(heading_to_u8(level));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(level) = heading_level.take() {
                    blocks.push(Block::Heading {
                        level,
                        text: std::mem::take(&mut text).trim().to_string(),
                    });
                }
            }
            Event::Start(Tag::Paragraph) => text.clear(),
            Event::End(TagEnd::Paragraph) => {
                let t = std::mem::take(&mut text).trim().to_string();
                if !t.is_empty() && list_stack.is_empty() {
                    blocks.push(Block::Para(t));
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code = true;
                code_lines.clear();
                text.clear();
                let _ = matches!(kind, CodeBlockKind::Fenced(_) | CodeBlockKind::Indented);
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                let body = std::mem::take(&mut text);
                for line in body.lines() {
                    code_lines.push(line.to_string());
                }
                if !code_lines.is_empty() {
                    blocks.push(Block::Code(std::mem::take(&mut code_lines)));
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                in_quote = true;
                text.clear();
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                in_quote = false;
                let t = std::mem::take(&mut text).trim().to_string();
                if !t.is_empty() {
                    blocks.push(Block::Quote(t));
                }
            }
            Event::Start(Tag::List(start)) => list_stack.push(start),
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => text.clear(),
            Event::End(TagEnd::Item) => {
                let t = std::mem::take(&mut text).trim().to_string();
                let depth = list_stack.len().saturating_sub(1) as u8;
                let marker = match list_stack.last_mut() {
                    Some(Some(n)) => {
                        let m = format!("{n}.");
                        *list_stack.last_mut().unwrap() = Some(*n + 1);
                        m
                    }
                    _ => "\u{2022}".to_string(),
                };
                if !t.is_empty() {
                    blocks.push(Block::Bullet { depth, marker, text: t });
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                pending_image = Some(dest_url.to_string());
                text.clear();
            }
            Event::End(TagEnd::Image) => {
                if let Some(src) = pending_image.take() {
                    blocks.push(Block::Image {
                        src,
                        alt: std::mem::take(&mut text).trim().to_string(),
                    });
                }
            }
            Event::Rule => blocks.push(Block::Rule),
            Event::Start(Tag::Table(_)) => {
                table_rows.clear();
                table_row.clear();
                text.clear();
            }
            Event::End(TagEnd::Table) => {
                let widths = column_widths(&table_rows);
                for (i, row) in table_rows.iter().enumerate() {
                    let line = format_table_row(row, &widths);
                    if !line.trim().is_empty() {
                        blocks.push(Block::TableRow {
                            text: line,
                            header: i == 0,
                        });
                    }
                }
                table_rows.clear();
            }
            Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                table_row.clear();
            }
            Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                table_rows.push(std::mem::take(&mut table_row));
            }
            Event::Start(Tag::TableCell) => text.clear(),
            Event::End(TagEnd::TableCell) => {
                table_row.push(std::mem::take(&mut text).trim().to_string());
            }
            Event::Text(s) | Event::Code(s) => {
                if in_code {
                    text.push_str(&s);
                } else {
                    text.push_str(&s);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_code {
                    text.push('\n');
                } else {
                    text.push(' ');
                }
            }
            Event::TaskListMarker(done) => {
                text.push_str(if done { "[x] " } else { "[ ] " });
            }
            _ => {}
        }
        let _ = in_quote;
    }
    blocks
}

fn heading_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Estimate the rendered width of `text` in points.
fn est_width(text: &str, size: f32, factor: f32) -> f32 {
    text.chars().count() as f32 * size * factor
}

/// Word-wrap `text` so that no line exceeds `max_w` points (estimated).
fn wrap(text: &str, size: f32, factor: f32, max_w: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if est_width(&candidate, size, factor) > max_w && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Compute the display width of each table column (in characters).
fn column_widths(rows: &[Vec<String>]) -> Vec<usize> {
    let mut widths: Vec<usize> = Vec::new();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            let w = cell.chars().count();
            if i < widths.len() {
                widths[i] = widths[i].max(w);
            } else {
                widths.push(w);
            }
        }
    }
    widths
}

/// Render a table row as a fixed-width, pipe-separated line for monospace output.
fn format_table_row(row: &[String], widths: &[usize]) -> String {
    let mut out = String::new();
    for (i, cell) in row.iter().enumerate() {
        let w = widths.get(i).copied().unwrap_or_else(|| cell.chars().count());
        let pad = w.saturating_sub(cell.chars().count());
        out.push_str(cell);
        out.extend(std::iter::repeat(' ').take(pad));
        if i + 1 < row.len() {
            out.push_str("  |  ");
        }
    }
    out
}

/// Hard-wrap a string to at most `max_chars` characters per line (for code).
fn hard_wrap(text: &str, max_chars: usize) -> Vec<String> {
    if text.chars().count() <= max_chars {
        return vec![text.to_string()];
    }
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(max_chars)
        .map(|c| c.iter().collect())
        .collect()
}

/// Read the first readable font from `candidates`.
fn read_first(candidates: &[&str]) -> Option<Vec<u8>> {
    candidates.iter().find_map(|p| std::fs::read(p).ok())
}

/// Locate and load a regular Unicode TrueType font for embedding.
fn load_regular_font(explicit: Option<&Path>) -> Option<Vec<u8>> {
    if let Some(p) = explicit {
        if let Ok(b) = std::fs::read(p) {
            return Some(b);
        }
    }
    read_first(&[
        // Linux
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        // macOS
        "/Library/Fonts/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        // Windows
        "C:/Windows/Fonts/arial.ttf",
        "C:/Windows/Fonts/segoeui.ttf",
    ])
}

/// Locate and load a monospace TrueType font for code blocks.
fn load_mono_font() -> Option<Vec<u8>> {
    read_first(&[
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        "/Library/Fonts/Courier New.ttf",
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
        "C:/Windows/Fonts/consola.ttf",
        "C:/Windows/Fonts/cour.ttf",
    ])
}

fn no_font_message() -> String {
    "no embeddable TrueType font found. Install a font such as DejaVu Sans, or \
     pass one explicitly with --font <path-to.ttf>"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_long_text() {
        let lines = wrap("aaaa bbbb cccc dddd eeee", 12.0, 0.5, 60.0);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|l| !l.is_empty()));
    }

    #[test]
    fn hard_wrap_splits_code() {
        let lines = hard_wrap("abcdefghij", 4);
        assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn parses_heading_and_paragraph() {
        let blocks = parse_blocks("## Title\n\nSome body text.\n");
        assert!(matches!(blocks[0], Block::Heading { level: 2, .. }));
        assert!(blocks.iter().any(|b| matches!(b, Block::Para(_))));
    }

    #[test]
    fn parses_list_items_with_markers() {
        let blocks = parse_blocks("## L\n\n- one\n- two\n\n1. first\n2. second\n");
        let bullets: Vec<&Block> = blocks
            .iter()
            .filter(|b| matches!(b, Block::Bullet { .. }))
            .collect();
        assert_eq!(bullets.len(), 4);
        if let Block::Bullet { marker, .. } = bullets[2] {
            assert_eq!(marker, "1.");
        } else {
            panic!("expected ordered marker");
        }
    }

    #[test]
    fn parses_code_block() {
        let blocks = parse_blocks("## C\n\n```\nline1\nline2\n```\n");
        assert!(blocks.iter().any(|b| matches!(b, Block::Code(l) if l.len() == 2)));
    }

    #[test]
    fn column_widths_take_the_max_per_column() {
        let rows = vec![
            vec!["a".to_string(), "long".to_string()],
            vec!["ccc".to_string(), "x".to_string()],
        ];
        assert_eq!(column_widths(&rows), vec![3, 4]);
    }

    #[test]
    fn format_table_row_pads_and_joins_columns() {
        let widths = vec![3, 4];
        let row = vec!["a".to_string(), "x".to_string()];
        let line = format_table_row(&row, &widths);
        assert_eq!(line, "a    |  x   ");
    }

    #[test]
    fn parses_table_into_rows() {
        let md = "## T\n\n| A | B |\n| - | - |\n| 1 | 2 |\n";
        let blocks = parse_blocks(md);
        let rows: Vec<_> = blocks
            .iter()
            .filter(|b| matches!(b, Block::TableRow { .. }))
            .collect();
        assert_eq!(rows.len(), 2, "header + one body row expected");
        assert!(matches!(rows[0], Block::TableRow { header: true, .. }));
    }
}
