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
    padding: 24px
    background-color: white

    H1 {
        font-size: 48px
        font-weight: 800
        color: #111111
        "Hello"
    }

    Paragraph {
        color: #666666
        line-height: 1.6
        "This was written in HML."
    }

    Link[href: "https://example.com"] {
        color: #4f46e5
        "Read more"
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
- Handle text nodes as direct element content
- Handle attributes in bracket syntax like `Meta[charset: "utf-8"]`
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
    color: #111111
    font-size: 16px
    "Hello"
}
```

becomes a paragraph element with a generated class, and the CSS is written separately.

Attributes are written explicitly in brackets after the element name, for example `Link[href: "https://example.com"]` or `Input[type: "email", required]`.

## Diagnostics

The compiler reports errors and warnings with file and line information.

Typical examples include:

- missing required attributes like `href` on links
- syntax issues from malformed input
- unknown properties that are passed through

The diagnostics are readable enough for debugging example files, though there is still room to improve the wording and formatting.

## Things that still need work

A few obvious next steps:

- Compiler support for the finalized bracket-attribute syntax
- Better parsing for fully natural multi-part CSS values
- Cleaner HTML formatting in generated files
- More precise diagnostics
- Tests

## Why this exists

Mostly because markup can get noisy, and it's interesting to explore whether a block-based syntax can feel nicer without giving up the simplicity of static HTML and CSS.

This project is a compiler experiment first, and a polished tool second.
