# rpres Demo Presentation

A tour of every Markdown feature rpres understands.

Press **space**, **→**, or **click** to advance — press **a** to toggle animation mode.

Press **backspace** or **←** to go back.

## Text Formatting

This paragraph shows **bold**, *italic*, ***bold italic***, and ~~strikethrough~~ text.

Inline `code` looks like this, and you can mix `formatting` inside sentences.

Here is a second block that appears separately in animation mode.

## Lists

Unordered list:

- First item
- Second item
  - Nested item
  - Another nested item
- Third item

Ordered list:

1. Step one
2. Step two
3. Step three

## Task Lists

- [x] Implement slide parser
- [x] Add HTML templates
- [ ] Conquer the world

## Code Blocks

A fenced Rust code block (note the `# comment` does not split slides):

```rust
fn main() {
    // # this hash should not start a new slide
    println!("Hello from rpres!");
}
```

A shell example:

```sh
rpres demo.md --open --template modern
```

## Tables

| Template  | Style            | Best for          |
|-----------|------------------|-------------------|
| terminal  | green monospace  | hacker vibes      |
| classic   | serif, beamer    | academic talks    |
| modern    | gradient, clean  | product demos     |

## Blockquotes & Links

> "Simplicity is the ultimate sophistication."
> — Leonardo da Vinci

Visit the [project repository](https://example.com/rpres) for more.

## Images

Images scale to fit the slide:

![A placeholder image](https://placehold.co/600x200/png)

## Animation Mode

When animation mode is on (`--click` or the `a` key):

Each non-empty block waits for a click.

Like this one.

And this final one.

## Thank You

Thanks for trying **rpres**!

- Built with Rust, clap, tiny_http and pulldown-cmark
- Three templates, paged & single-page modes
- Keyboard and mouse navigation
