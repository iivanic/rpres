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
- **PDF export** (`--export`) — *experimental*; see below.

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
| `-e`, `--export <PDF>` | **[Experimental]** Export to PDF and exit |
| `--font <TTF>` | TrueType font to embed in the exported PDF |
| `--no-pdfa` | Disable PDF/A-1b conformance tagging when exporting |
| `-s`, `--server <IP>` | Bind address (default `127.0.0.1`) |
| `--port <PORT>` | Bind port (default `8080`) |

## Writing slides

- A line starting with `# ` or `## ` begins a new slide.
- The first `#` heading is used as the document/presentation title.
- Standard Markdown is supported: text formatting, lists, task lists, fenced
  code blocks, tables, blockquotes, links, and images.

## PDF export (experimental)

> **⚠️ Experimental.** The PDF exporter is functional but still rough around
> the edges. Output may change between versions and some content can be
> clipped or simplified. Use it for drafts and review, not final artifacts.

```sh
rpres demo.md --export demo.pdf            # one slide per page, PDF/A-1b
rpres demo.md -e out.pdf --font My.ttf     # embed a specific TrueType font
rpres demo.md -e out.pdf --no-pdfa         # plain PDF, no archival tagging
```

What's rendered: headings, paragraphs (word-wrapped), bullet/ordered/task
lists with nesting, monospace code blocks, tables (column-aligned), blockquotes,
horizontal rules, and embedded local images. Fonts are subset and embedded so
Unicode glyphs render correctly.

### Known limitations

- PDF/A-1b tagging is **best-effort** — strict validator conformance is not
  guaranteed.
- Only **local** images (relative to the Markdown file) are embedded; remote
  (`http`/`https`) and `data:` images show a `[image: …]` placeholder.
- **One slide per page** — content that overflows a page is clipped.
- Inline **bold/italic** is flattened to plain text in the PDF.

## Development

```sh
cargo build      # build
cargo test       # run unit + integration tests
cargo run -- demo.md --open
```

## License

MIT
