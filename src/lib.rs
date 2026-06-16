//! rpres — render a Markdown file as a browser presentation.

pub mod cli;
pub mod export;
pub mod render;
pub mod server;
pub mod slides;
pub mod templates;

pub use cli::Cli;
pub use slides::{Slide, parse_slides};
