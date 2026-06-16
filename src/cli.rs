//! Command line interface definition.

use clap::Parser;
use std::path::PathBuf;

/// Render a Markdown file as a presentation in the browser.
#[derive(Parser, Debug)]
#[command(name = "rpres", version, about, long_about = None)]
pub struct Cli {
    /// Path to the Markdown presentation file.
    #[arg(required_unless_present = "list_templates")]
    pub file: Option<PathBuf>,

    /// Animation mode: reveal every non-empty block on space / mouse click.
    #[arg(short = 'c', long = "click")]
    pub click: bool,

    /// Open the presentation in the default browser.
    #[arg(short = 'o', long = "open")]
    pub open: bool,

    /// List the available HTML templates and exit.
    #[arg(short = 'l', long = "list-templates")]
    pub list_templates: bool,

    /// HTML template to use (terminal, classic, modern).
    #[arg(short = 't', long = "template", default_value = "modern")]
    pub template: String,

    /// Paged mode: load each slide from the server via AJAX.
    #[arg(short = 'p', long = "paged")]
    pub paged: bool,

    /// IP address to bind the server to.
    #[arg(short = 's', long = "server", default_value = "127.0.0.1")]
    pub server: String,

    /// Port to bind the server to.
    #[arg(long = "port", default_value_t = 8080)]
    pub port: u16,
}
