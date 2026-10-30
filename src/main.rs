use clap::Parser;
use rpres::cli::Cli;
use rpres::server::{Deck, serve};
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

    let deck = match Deck::new(path, &cli.template, cli.click) {
        Ok(deck) => deck,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
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
        deck.slide_count(),
        cli.template,
        url
    );
    println!(
        "rpres: watching '{}' — the browser reloads when it changes",
        path.display()
    );
    println!(
        "rpres: press 'a' to toggle animation, '+/-/0' to resize text, \
         arrows/space to navigate, Ctrl+C to quit"
    );

    if cli.open {
        open_browser(&url);
    }

    if let Err(err) = serve(&addr, deck) {
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
