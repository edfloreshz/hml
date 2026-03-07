# HML

HML is a small Rust compiler for a block-based markup language that compiles to plain HTML and CSS.

The goal is simple:

- keep the document model close to HTML
- make authoring cleaner and easier to scan
- let structure, attributes, text, and styling live together
- emit static output with no browser runtime

The project is still experimental, but it already supports compiling real example pages into usable HTML and CSS files.

## What HML looks like

Here is a small example:

```hml
Document {
    Head {
        Meta[charset: "utf-8"]
        Title "Hello HML"
    }

    Body {
        Box[class: "page"] {
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
                text-decoration: none

                "Read more"
            }
        }
    }
}
```

That compiles into:

- an HTML document with normal tags
- a generated CSS file with scoped classes
- an injected stylesheet link
- an HTML5 doctype when the top-level document is `Document`

## Core syntax

HML supports three main concepts inside an element:

1. **Attributes**
   - written in brackets after the element name
   - example: `Link[href: "https://example.com"]`

2. **Style declarations**
   - written inside the block body
   - example: `padding: 24px`

3. **Child content**
   - nested elements
   - text nodes written as string literals

### Inline text shorthand

Elements can also take direct inline text:

```hml
Title "Acme UI"
Paragraph "Hello world"
Link[href: "https://example.com"] "Read more"
```

### Multiline attributes

Bracket attributes can span multiple lines:

```hml
Input[
    id: "email",
    name: "email",
    type: "email",
    placeholder: "you@example.com"
] {
    width: 100%
}
```

### Multi-part CSS values

CSS values can contain multiple tokens:

```hml
Box {
    margin: "0 auto"
    padding: "0 40px 40px 40px"
    border: "1px solid #e2e8f0"
}
```

## Current compiler features

Right now the compiler can:

- tokenize and parse `.hml` files
- parse bracket attributes
- parse inline text shorthand
- parse text nodes inside blocks
- parse multi-token CSS values
- map HML element names to HTML tags
- distinguish element attributes from CSS declarations
- generate HTML and CSS files
- generate scoped CSS classes from element styles
- merge generated classes with user-provided `class` attributes
- inject a stylesheet link into generated HTML
- emit `<!DOCTYPE html>` for HML documents rooted at `Document`
- compile a single file or a full directory from the CLI
- preserve directory structure when compiling directories
- report diagnostics with file and line information

There are several example files in `examples/` that exercise realistic layouts and are useful for validating output.

## Building

Make sure you have a recent Rust toolchain installed, then run:

```sh
cargo build
```

## Installation

Install the CLI using Cargo:

```sh
cargo install --path .
```

## CLI usage

Compile a single file:

```sh
hml compile examples/article.hml
```

Compile a directory:

```sh
hml compile examples
```

Watch and rebuild on changes:

```sh
hml watch examples
```

Start the dev server with live reload:

```sh
hml dev
```

You can also point it at a specific file or directory:

```sh
hml dev examples
```

The `dev` command serves the compiled output, watches for `.hml` changes, live reloads the browser after successful rebuilds, and opens the browser automatically.

The input path defaults to the current directory, and the output directory defaults to `dist`, so both can be omitted.

You can still override the default output directory:

```sh
hml dev examples --out build --port 4000
```

The compiler writes:

- one `.html` file
- one `.css` file

for each input `.hml` file.

## Example output model

Given this HML:

```hml
Paragraph {
    color: #111111
    font-size: 16px

    "Hello"
}
```

the compiler emits:

- a `<p>` element
- a generated CSS class like `hml-xxxxxx`
- a CSS rule containing the declarations

If the element already has a user class, both classes are preserved.

## HML element mapping

HML uses descriptive element names instead of raw HTML tag names. For example:

- `Document` -> `html`
- `Head` -> `head`
- `Body` -> `body`
- `Title` -> `title`
- `Meta` -> `meta`
- `Box` -> `div`
- `Section` -> `section`
- `Article` -> `article`
- `Header` -> `header`
- `Footer` -> `footer`
- `Main` -> `main`
- `Paragraph` -> `p`
- `Span` -> `span`
- `Link` -> `a`
- `Image` -> `img`
- `List` -> `ul`
- `OrderedList` -> `ol`
- `ListItem` -> `li`
- `Form` -> `form`
- `Input` -> `input`
- `TextArea` -> `textarea`
- `Select` -> `select`
- `Option` -> `option`
- `Button` -> `button`
- `Label` -> `label`
- `Table` -> `table`
- `TableHead` -> `thead`
- `TableBody` -> `tbody`
- `TableRow` -> `tr`
- `TableHeader` -> `th`
- `TableCell` -> `td`

There are more mappings in the source.

## Styling model

Styles are written directly on elements in the block body.

Example:

```hml
Box {
    background-color: white
    padding: 24px
    border-radius: 16px
}
```

During code generation, the compiler:

1. collects the CSS declarations
2. generates a stable scoped class name
3. attaches that class to the output HTML
4. writes the declarations into the generated CSS file

Attributes are not treated as styles. They are parsed separately from bracket syntax and passed through to HTML output.

## Diagnostics

The compiler reports errors and warnings with file and line information.

Typical diagnostics include:

- missing required attributes like `href` on `Link`
- missing required attributes like `src` on `Image` or `Script`
- syntax errors from malformed input
- unknown properties or attributes

## Project status

The compiler now supports the current example syntax, including:

- bracket attributes
- inline text shorthand
- text nodes
- multi-part CSS values

There is still room to improve, especially in:

- HTML formatting polish
- diagnostics wording and recovery
- broader HTML element coverage
- validation rules for invalid combinations on void elements
- more exhaustive integration tests

## Why this exists

This project exists to explore whether markup can feel cleaner to author without giving up the simplicity of static HTML and CSS.

It is a compiler experiment first, and a polished tool second.
