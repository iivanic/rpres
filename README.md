# rpres

A small, dependency-light CLI tool that renders a Markdown file as a
presentation in your browser. Each `#` (or `##`) heading starts a new slide,
and a built-in HTTP server serves the deck with keyboard and click navigation.

## Features

- **Markdown → slides** — splits on `#` / `##` headings; deeper headings and
  `#` inside fenced code blocks are kept inside the slide.
- **Three themes** — `terminal`, `classic`, and `modern` (default).
- **Keyboard & click navigation** — see [Controls](#controls) below.
- **Animation mode** (`--click`) — reveal each block, and list items one at a
  time, on space / mouse click.
- **Hot reload** — edit the Markdown while serving and the open browser tab
  reloads by itself, staying on the slide you were looking at.
- **Text size control** — `+`, `-` and `0` resize the deck on the fly.
- **Local images** — files next to the Markdown source (images, fonts, …) are
  served by the built-in server, so relative paths just work.
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
| `-s`, `--server <IP>` | Bind address (default `127.0.0.1`) |
| `--port <PORT>` | Bind port (default `8080`) |

## Writing slides

- A line starting with `# ` or `## ` begins a new slide.
- The first `#` heading is used as the document/presentation title.
- Standard Markdown is supported: text formatting, lists, task lists, fenced
  code blocks, tables, blockquotes, links, and images.

### Images

Images are served by rpres itself, relative to the directory of the Markdown
file. For a deck at `talks/demo.md`, `![logo](img/logo.png)` is looked up at
`talks/img/logo.png`; remote `http(s)://` URLs are loaded by the browser as
usual.

## Controls

Navigate the running presentation with the keyboard or the mouse:

| Key | Action |
| --- | --- |
| `→`, `Space`, `Enter`, `PageDown`, `l` | Next slide / reveal next block |
| `←`, `Backspace`, `PageUp`, `h` | Previous slide / hide last block |
| `Home` | Jump to the first slide |
| `End` | Jump to the last slide |
| `a` | Toggle animation (click-to-reveal) mode |
| `f` | Toggle fullscreen |
| `+`, `=` | Increase the text size |
| `-`, `_` | Decrease the text size |
| `0` | Reset the text size to the default |
| Mouse click | Advance to the next slide / block |

The text size is a multiplier on the theme's font size and is shown in the
page counter while it is not 100%. It survives the automatic reloads, just
like the current slide.

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
fragments, and hide the on-screen HUD.

## Development

```sh
cargo build      # build
cargo test       # run unit + integration tests
cargo run -- demo.md --open
```

## License

MIT
