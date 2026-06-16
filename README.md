# rpres

A small, dependency-light CLI tool that renders a Markdown file as a
presentation in your browser. Each `#` (or `##`) heading starts a new slide,
and a built-in HTTP server serves the deck with keyboard and click navigation.

## Features

- **Markdown → slides** — splits on `#` / `##` headings; deeper headings and
  `#` inside fenced code blocks are kept inside the slide.
- **Three themes** — `terminal`, `classic`, and `modern` (default).
- **Keyboard & click navigation** — arrows, `PageUp`/`PageDown`, `h`/`l`,
  `Space`, `Home`/`End`, and `Backspace` to go back.
- **Animation mode** (`--click`) — reveal each block, and list items one at a
  time, on space / mouse click.
- **Paged mode** (`--paged`) — load each slide on demand from the server.
- **Auto-open** (`--open`) — launch the deck in your default browser
  (Linux/BSD `xdg-open`, macOS `open`, Windows `explorer`).
- **Print to PDF** — the served deck carries print-friendly CSS, so
  **Ctrl/Cmd+P → "Save as PDF"** produces one slide per page; see below.

## Installation

```sh
cargo build --release
# binary at target/release/rpres
```

## Usage

```sh
rpres demo.md                       # serve on http://127.0.0.1:8080
rpres demo.md --open                # serve and open the browser
rpres demo.md -t terminal           # use the terminal theme
rpres demo.md --click               # reveal blocks on click/space
rpres demo.md --paged               # load slides on demand
rpres demo.md -s 0.0.0.0 --port 9000  # bind address / port
rpres --list-templates              # list available themes
```

### Options

| Flag | Description |
| --- | --- |
| `<FILE>` | Markdown presentation file (required unless `--list-templates`) |
| `-c`, `--click` | Animation mode: reveal blocks/list items on click or space |
| `-o`, `--open` | Open the presentation in the default browser |
| `-l`, `--list-templates` | List available HTML templates and exit |
| `-t`, `--template <NAME>` | Theme: `terminal`, `classic`, `modern` (default) |
| `-p`, `--paged` | Paged mode: load each slide via AJAX |
| `-s`, `--server <IP>` | Bind address (default `127.0.0.1`) |
| `--port <PORT>` | Bind port (default `8080`) |

## Writing slides

- A line starting with `# ` or `## ` begins a new slide.
- The first `#` heading is used as the document/presentation title.
- Standard Markdown is supported: text formatting, lists, task lists, fenced
  code blocks, tables, blockquotes, links, and images.

## Print to PDF

rpres relies on your **browser's** print engine for PDF output — this gives
full-fidelity styling, real tables, web fonts, syntax colors, and selectable,
zoomable text, exactly as they appear on screen.

The served deck always carries a print stylesheet, so you can turn any
presentation into a PDF directly from the browser:

1. Serve and open the deck, e.g. `rpres demo.md --open`.
2. Open the print dialog with **Ctrl+P** (macOS: **Cmd+P**).
3. Choose **"Save as PDF"** as the destination.
4. Set margins to **None** and enable **Background graphics** for best results.

The `@media print` rules make each slide one 16:9 page, reveal all animation
fragments, and hide the on-screen HUD. Print from a normal (non-`--paged`)
deck so the whole presentation is in the page.

> **Note:** `--paged` is **not** suitable for printing to PDF. In paged mode
> only the current slide is loaded in the browser, so the print output would
> contain just that one slide. Serve the deck without `--paged` before
> saving to PDF.

## Development

```sh
cargo build      # build
cargo test       # run unit + integration tests
cargo run -- demo.md --open
```

## License

MIT
