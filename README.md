# HML

HML is a small Rust compiler for a markup language that looks a bit like HTML, but uses block syntax with curly braces and lets you write styling directly on elements.

The rough idea is:

- Keep the structure of HTML
- Make dev experience a little cleaner
- Compile everything down to plain HTML + CSS
- Avoid any browser runtime

This project is still early, but it already works well enough to compile files and inspect the generated output.

## What HML looks like

Here’s a small example:

```hml
Box {
    padding: 24
    background-color: white

    H1 {
        content: "Hello"
        font-size: 48
        font-weight: 800
        color: #111111
    }

    Paragraph {
        content: "This was written in HML."
        color: #666666
        line-height: 1.6
    }

    Link {
        href: "https://example.com"
        content: "Read more"
        color: #4f46e5
    }
}
```

That compiles into normal HTML and a generated CSS file with scoped classes.

## Current status

Right now the compiler can:

- Tokenize and parse `.hml` files
- Map HML element names to HTML tags
- Distinguish HTML attributes from CSS properties
- Generate HTML and CSS files
- Handle `content` as text content for normal elements
- Handle `content` as an attribute for `Meta`
- Compile a single file or a complete directory from the CLI
- Inject a stylesheet link into the generated HTML
- Preserve original directory structure

There are also several example files in `examples/` that are useful for testing output.

## Building

Make sure you have a recent Rust toolchain installed, then run:

```sh
cargo build
```

## CLI usage

Compile a single file:

```sh
cargo run -- compile examples/blog/blog.hml --out dist
```

Compile a directory of examples:

```sh
cargo run -- compile examples --out dist/examples
```

The compiler writes:

- one `.html` file
- one `.css` file

for each input `.hml` file.

## HML element mapping

HML uses descriptive element names instead of raw HTML tag names. For example:

- `Document` -> `html`
- `Head` -> `head`
- `Body` -> `body`
- `Box` -> `div`
- `Paragraph` -> `p`
- `Link` -> `a`
- `Image` -> `img`

There are more supported mappings in the compiler source.

## Styling model

CSS properties are written directly on elements. During code generation, those styles are emitted into CSS rules and associated with generated class names.

For example:

```hml
Paragraph {
    content: "Hello"
    color: #111111
    font-size: 16
}
```

becomes a paragraph element with a generated class, and the CSS is written separately.

Attributes that are not recognized as CSS are treated as HTML attributes.

## Diagnostics

The compiler reports errors and warnings with file and line information.

Typical examples include:

- missing required attributes like `href` on links
- syntax issues from malformed input
- unknown properties that are passed through

The diagnostics are readable enough for debugging example files, though there is still room to improve the wording and formatting.

## Things that still need work

A few obvious next steps:

- Better parsing for fully natural multi-part CSS values
- Cleaner HTML formatting in some generated files
- More precise diagnostics
- Documentation for the language itself
- Tests

## Why this exists

Mostly because markup can get noisy, and it's interesting to explore whether a block-based syntax can feel nicer without giving up the simplicity of static HTML and CSS.

This project is a compiler experiment first, and a polished tool second.
