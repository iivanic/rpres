use clap::Parser;
use rpres::cli::Cli;
use rpres::export::{ExportOptions, write_pdf_file};
use rpres::render::{build_page, slide_html};
use rpres::server::{App, serve};
use rpres::slides::parse_slides;
use rpres::templates::{TEMPLATES, theme_css};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.list_templates {
        println!("Available templates:");
        for name in TEMPLATES {
            println!("  {name}");
        }
        return ExitCode::SUCCESS;
    }

    if theme_css(&cli.template).is_none() {
        eprintln!(
            "error: unknown template '{}'. Available: {}",
            cli.template,
            TEMPLATES.join(", ")
        );
        return ExitCode::FAILURE;
    }

    let path = match &cli.file {
        Some(p) => p,
        None => {
            eprintln!("error: a Markdown file is required");
            return ExitCode::FAILURE;
        }
    };

    let markdown = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("error: could not read '{}': {err}", path.display());
            return ExitCode::FAILURE;
        }
    };

    let slides = parse_slides(&markdown);
    if slides.is_empty() {
        eprintln!("error: '{}' contains no slides", path.display());
        return ExitCode::FAILURE;
    }

    let title = slides
        .iter()
        .find(|s| s.is_title())
        .map(|s| s.title.clone())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "rpres".to_string())
        });

    if let Some(out) = &cli.export {
        let base_dir = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let opts = ExportOptions {
            title: title.clone(),
            pdfa: !cli.no_pdfa,
            base_dir,
            font_path: cli.font.clone(),
        };
        match write_pdf_file(&slides, out, &opts) {
            Ok(()) => {
                println!(
                    "rpres: exported {} slide(s) to {}{}",
                    slides.len(),
                    out.display(),
                    if opts.pdfa { " (PDF/A-1b)" } else { "" }
                );
                return ExitCode::SUCCESS;
            }
            Err(err) => {
                eprintln!("error: export failed: {err}");
                return ExitCode::FAILURE;
            }
        }
    }

    let page = build_page(&title, &cli.template, &slides, cli.click, cli.paged);
    let slide_fragments = if cli.paged {
        slides.iter().map(slide_html).collect()
    } else {
        Vec::new()
    };

    let addr = format!("{}:{}", cli.server, cli.port);
    let browse_host = if cli.server == "0.0.0.0" {
        "127.0.0.1"
    } else {
        cli.server.as_str()
    };
    let url = format!("http://{browse_host}:{}", cli.port);

    println!(
        "rpres: {} slide(s), template '{}', serving on {}",
        slides.len(),
        cli.template,
        url
    );
    if cli.paged {
        println!("rpres: paged mode (slides loaded over AJAX)");
    }
    println!("rpres: press 'a' to toggle animation, arrows/space to navigate, Ctrl+C to quit");

    if cli.open {
        open_browser(&url);
    }

    let app = App {
        page,
        slides: slide_fragments,
        paged: cli.paged,
    };

    if let Err(err) = serve(&addr, app) {
        eprintln!("error: server failed: {err}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

/// Open `url` in the system default browser, ignoring failures.
fn open_browser(url: &str) {
    // Linux and the BSDs use the freedesktop `xdg-open` helper.
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
    ))]
    let program = "xdg-open";
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "windows")]
    let program = "explorer";

    let _ = std::process::Command::new(program).arg(url).spawn();
}
