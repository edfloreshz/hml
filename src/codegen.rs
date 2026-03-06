use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::hash::{Hash, Hasher};

use crate::ast::{Document, ElementNode, Node, Property, Value};
use crate::diagnostics::{Diagnostic, Diagnostics, SourceLocation};

#[derive(Debug, Clone)]
pub struct GeneratedClass {
    pub name: String,
    pub declarations: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct CodegenOutput {
    pub html: String,
    pub css: String,
    pub classes: Vec<GeneratedClass>,
}

pub fn generate(
    document: &Document,
    file_name: impl Into<String>,
    diagnostics: &mut Diagnostics,
) -> CodegenOutput {
    let mut generator = CodeGenerator::new(file_name.into(), diagnostics);
    generator.generate(document)
}

struct CodeGenerator<'a> {
    file_name: String,
    diagnostics: &'a mut Diagnostics,
    classes: BTreeMap<String, BTreeMap<String, String>>,
}

impl<'a> CodeGenerator<'a> {
    fn new(file_name: String, diagnostics: &'a mut Diagnostics) -> Self {
        Self {
            file_name,
            diagnostics,
            classes: BTreeMap::new(),
        }
    }

    fn generate(&mut self, document: &Document) -> CodegenOutput {
        let mut html = String::new();

        for (index, node) in document.nodes.iter().enumerate() {
            self.write_node(node, 0, &mut html);

            if index + 1 < document.nodes.len() {
                html.push('\n');
            }
        }

        let mut css = String::new();
        let mut rendered_classes = Vec::new();

        for (index, (class_name, declarations)) in self.classes.iter().enumerate() {
            if index > 0 {
                css.push('\n');
                css.push('\n');
            }

            writeln!(css, ".{} {{", class_name).unwrap();

            for (property, value) in declarations {
                writeln!(css, "    {}: {};", property, value).unwrap();
            }

            write!(css, "}}").unwrap();

            rendered_classes.push(GeneratedClass {
                name: class_name.clone(),
                declarations: declarations.clone(),
            });
        }

        CodegenOutput {
            html,
            css,
            classes: rendered_classes,
        }
    }

    fn write_node(&mut self, node: &Node, indent: usize, out: &mut String) {
        self.write_element(&node.element, indent, out);
    }

    fn write_element(&mut self, element: &ElementNode, indent: usize, out: &mut String) {
        let html_tag = map_element_name(&element.name);
        let analyzed = self.analyze_element(element);

        let indent_str = "    ".repeat(indent);

        write!(out, "{}<{}", indent_str, html_tag).unwrap();

        let rendered_attrs =
            self.render_attributes(analyzed.generated_class.as_deref(), analyzed.attributes);

        if !rendered_attrs.is_empty() {
            write!(out, " {}", rendered_attrs.join(" ")).unwrap();
        }

        if is_void_element(html_tag) {
            out.push('>');
            return;
        }

        out.push('>');

        let has_text = analyzed.text_content.is_some();
        let has_children = !element.children.is_empty();

        match (has_text, has_children) {
            (false, false) => {
                write!(out, "</{}>", html_tag).unwrap();
            }
            (true, false) => {
                out.push_str(&escape_html_text(
                    analyzed.text_content.as_deref().unwrap_or_default(),
                ));
                write!(out, "</{}>", html_tag).unwrap();
            }
            (false, true) => {
                out.push('\n');

                for (index, child) in element.children.iter().enumerate() {
                    self.write_node(child, indent + 1, out);

                    if index + 1 < element.children.len() {
                        out.push('\n');
                    }
                }

                out.push('\n');
                write!(out, "{}</{}>", indent_str, html_tag).unwrap();
            }
            (true, true) => {
                out.push('\n');
                write!(
                    out,
                    "{}    {}",
                    indent_str,
                    escape_html_text(analyzed.text_content.as_deref().unwrap_or_default())
                )
                .unwrap();

                for child in &element.children {
                    out.push('\n');
                    self.write_node(child, indent + 1, out);
                }

                out.push('\n');
                write!(out, "{}</{}>", indent_str, html_tag).unwrap();
            }
        }
    }

    fn analyze_element(&mut self, element: &ElementNode) -> AnalyzedElement {
        let mut attributes = BTreeMap::new();
        let mut styles = BTreeMap::new();
        let mut text_content = None;

        for property in &element.properties {
            let key = property.name.trim().to_string();
            let raw_value = property.value.as_str().to_string();

            if key == "content" && element.name != "Meta" {
                text_content = Some(raw_value);
                continue;
            }

            if is_css_property(&key) {
                styles.insert(key, normalize_css_value(&property.name, &property.value));
                continue;
            }

            if is_known_html_attribute(&key) || is_likely_html_attribute(&key) {
                attributes.insert(key, raw_value);
                continue;
            }

            self.diagnostics.push(Diagnostic::unknown_property(
                self.location_for_property(property),
                element.name.clone(),
                &property.name,
            ));

            attributes.insert(key, raw_value);
        }

        if element.name == "Link" && !attributes.contains_key("href") {
            self.diagnostics
                .push(Diagnostic::missing_required_attribute(
                    self.location_for_element(element),
                    element.name.clone(),
                    "href",
                ));
        }

        if matches!(element.name.as_str(), "Image" | "Script") && !attributes.contains_key("src") {
            self.diagnostics
                .push(Diagnostic::missing_required_attribute(
                    self.location_for_element(element),
                    element.name.clone(),
                    "src",
                ));
        }

        let generated_class = if styles.is_empty() {
            None
        } else {
            let class_name = class_name_for(&element.name, &styles);
            self.classes
                .entry(class_name.clone())
                .or_insert_with(|| styles.clone());
            Some(class_name)
        };

        AnalyzedElement {
            attributes,
            text_content,
            generated_class,
        }
    }

    fn render_attributes(
        &self,
        generated_class: Option<&str>,
        mut attributes: BTreeMap<String, String>,
    ) -> Vec<String> {
        let provided_class = attributes.remove("class");
        let merged_class = merge_classes(generated_class, provided_class.as_deref());
        let meta_name = attributes.remove("name");
        let meta_content = attributes.remove("content");

        let mut rendered = Vec::new();

        if let Some(class_name) = merged_class {
            rendered.push(format!("class=\"{}\"", escape_html_attr(&class_name)));
        }

        if let Some(name) = meta_name {
            rendered.push(format!("name=\"{}\"", escape_html_attr(&name)));
        }

        if let Some(content) = meta_content {
            rendered.push(format!("content=\"{}\"", escape_html_attr(&content)));
        }

        for (name, value) in attributes {
            if is_boolean_attribute(&name) {
                if is_truthy_boolean_attr(&value) {
                    rendered.push(name);
                } else if is_falsy_boolean_attr(&value) {
                    continue;
                } else {
                    rendered.push(format!("{}=\"{}\"", name, escape_html_attr(&value)));
                }
            } else {
                rendered.push(format!("{}=\"{}\"", name, escape_html_attr(&value)));
            }
        }

        rendered
    }

    fn location_for_element(&self, element: &ElementNode) -> SourceLocation {
        SourceLocation::new(
            self.file_name.clone(),
            element.span.line.max(1),
            element.span.column.max(1),
        )
    }

    fn location_for_property(&self, property: &Property) -> SourceLocation {
        SourceLocation::new(
            self.file_name.clone(),
            property.span.line.max(1),
            property.span.column.max(1),
        )
    }
}

struct AnalyzedElement {
    attributes: BTreeMap<String, String>,
    text_content: Option<String>,
    generated_class: Option<String>,
}

fn class_name_for(element_name: &str, styles: &BTreeMap<String, String>) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    element_name.hash(&mut hasher);

    for (key, value) in styles {
        key.hash(&mut hasher);
        value.hash(&mut hasher);
    }

    let hash = hasher.finish() & 0xFF_FFFF;
    format!("hml-{:06x}", hash)
}

fn merge_classes(generated: Option<&str>, provided: Option<&str>) -> Option<String> {
    let mut classes = Vec::new();

    if let Some(generated) = generated {
        if !generated.trim().is_empty() {
            classes.push(generated.trim().to_string());
        }
    }

    if let Some(provided) = provided {
        for part in provided.split_whitespace() {
            if !part.is_empty() {
                classes.push(part.to_string());
            }
        }
    }

    let mut seen = BTreeSet::new();
    classes.retain(|class_name| seen.insert(class_name.clone()));

    if classes.is_empty() {
        None
    } else {
        Some(classes.join(" "))
    }
}

fn normalize_css_value(property_name: &str, value: &Value) -> String {
    let raw = value.as_str();

    if should_append_px(property_name, raw) {
        format!("{}px", raw)
    } else {
        raw.to_string()
    }
}

fn should_append_px(property_name: &str, raw: &str) -> bool {
    if is_unitless_css_property(property_name) {
        return false;
    }

    if raw == "0" || raw == "0.0" {
        return false;
    }

    if raw.contains(' ') {
        return false;
    }

    if raw.contains('%') || raw.chars().any(|ch| ch.is_ascii_alphabetic()) {
        return false;
    }

    is_numeric_literal(raw)
}

fn is_numeric_literal(raw: &str) -> bool {
    let value = raw.trim();
    if value.is_empty() {
        return false;
    }

    let body = if let Some(rest) = value.strip_prefix('-') {
        rest
    } else {
        value
    };

    !body.is_empty() && body.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
}

fn escape_html_text(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html_attr(input: &str) -> String {
    escape_html_text(input).replace('"', "&quot;")
}

fn map_element_name(name: &str) -> &str {
    match name {
        "Document" => "html",
        "Head" => "head",
        "Body" => "body",
        "Meta" => "meta",
        "Script" => "script",
        "Style" => "style",
        "Box" => "div",
        "Section" => "section",
        "Article" => "article",
        "Aside" => "aside",
        "Nav" => "nav",
        "Header" => "header",
        "Footer" => "footer",
        "Main" => "main",
        "Paragraph" => "p",
        "Span" => "span",
        "H1" => "h1",
        "H2" => "h2",
        "H3" => "h3",
        "H4" => "h4",
        "H5" => "h5",
        "H6" => "h6",
        "Link" => "a",
        "Image" => "img",
        "List" => "ul",
        "OrderedList" => "ol",
        "ListItem" => "li",
        "Form" => "form",
        "Input" => "input",
        "TextArea" => "textarea",
        "Select" => "select",
        "Option" => "option",
        "Button" => "button",
        "Label" => "label",
        "Table" => "table",
        "TableHead" => "thead",
        "TableBody" => "tbody",
        "TableRow" => "tr",
        "TableHeader" => "th",
        "TableCell" => "td",
        _ => name,
    }
}

fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn is_truthy_boolean_attr(value: &str) -> bool {
    matches!(value.trim(), "" | "true" | "True" | "TRUE")
}

fn is_falsy_boolean_attr(value: &str) -> bool {
    matches!(value.trim(), "false" | "False" | "FALSE")
}

fn is_boolean_attribute(name: &str) -> bool {
    matches!(
        name,
        "allowfullscreen"
            | "async"
            | "autofocus"
            | "autoplay"
            | "checked"
            | "controls"
            | "default"
            | "defer"
            | "disabled"
            | "formnovalidate"
            | "hidden"
            | "inert"
            | "ismap"
            | "itemscope"
            | "loop"
            | "multiple"
            | "muted"
            | "nomodule"
            | "novalidate"
            | "open"
            | "playsinline"
            | "readonly"
            | "required"
            | "reversed"
            | "selected"
    )
}

fn is_known_html_attribute(name: &str) -> bool {
    matches!(
        name,
        "accept"
            | "action"
            | "alt"
            | "charset"
            | "content"
            | "class"
            | "cols"
            | "colspan"
            | "dir"
            | "download"
            | "enctype"
            | "for"
            | "height"
            | "href"
            | "id"
            | "lang"
            | "max"
            | "maxlength"
            | "method"
            | "min"
            | "minlength"
            | "name"
            | "placeholder"
            | "rel"
            | "rows"
            | "rowspan"
            | "src"
            | "tabindex"
            | "target"
            | "title"
            | "type"
            | "value"
            | "width"
    )
}

fn is_likely_html_attribute(name: &str) -> bool {
    name.starts_with("data-")
        || name.starts_with("aria-")
        || name.starts_with("on")
        || name == "role"
        || name == "style"
}

fn is_unitless_css_property(name: &str) -> bool {
    matches!(
        name,
        "animation-iteration-count"
            | "aspect-ratio"
            | "column-count"
            | "fill-opacity"
            | "flex"
            | "flex-grow"
            | "flex-shrink"
            | "font-weight"
            | "grid-column"
            | "grid-row"
            | "line-height"
            | "opacity"
            | "order"
            | "orphans"
            | "scale"
            | "tab-size"
            | "widows"
            | "z-index"
            | "zoom"
    )
}

fn is_css_property(name: &str) -> bool {
    matches!(
        name,
        "align-content"
            | "align-items"
            | "align-self"
            | "animation"
            | "animation-delay"
            | "animation-direction"
            | "animation-duration"
            | "animation-fill-mode"
            | "animation-iteration-count"
            | "animation-name"
            | "animation-play-state"
            | "animation-timing-function"
            | "appearance"
            | "aspect-ratio"
            | "backdrop-filter"
            | "backface-visibility"
            | "background"
            | "background-attachment"
            | "background-blend-mode"
            | "background-clip"
            | "background-color"
            | "background-image"
            | "background-origin"
            | "background-position"
            | "background-repeat"
            | "background-size"
            | "block-size"
            | "border"
            | "border-block"
            | "border-block-color"
            | "border-block-end"
            | "border-block-end-color"
            | "border-block-end-style"
            | "border-block-end-width"
            | "border-block-start"
            | "border-block-start-color"
            | "border-block-start-style"
            | "border-block-start-width"
            | "border-block-style"
            | "border-block-width"
            | "border-bottom"
            | "border-bottom-color"
            | "border-bottom-left-radius"
            | "border-bottom-right-radius"
            | "border-bottom-style"
            | "border-bottom-width"
            | "border-collapse"
            | "border-color"
            | "border-end-end-radius"
            | "border-end-start-radius"
            | "border-image"
            | "border-image-outset"
            | "border-image-repeat"
            | "border-image-slice"
            | "border-image-source"
            | "border-image-width"
            | "border-inline"
            | "border-inline-color"
            | "border-inline-end"
            | "border-inline-end-color"
            | "border-inline-end-style"
            | "border-inline-end-width"
            | "border-inline-start"
            | "border-inline-start-color"
            | "border-inline-start-style"
            | "border-inline-start-width"
            | "border-inline-style"
            | "border-inline-width"
            | "border-left"
            | "border-left-color"
            | "border-left-style"
            | "border-left-width"
            | "border-radius"
            | "border-right"
            | "border-right-color"
            | "border-right-style"
            | "border-right-width"
            | "border-spacing"
            | "border-start-end-radius"
            | "border-start-start-radius"
            | "border-style"
            | "border-top"
            | "border-top-color"
            | "border-top-left-radius"
            | "border-top-right-radius"
            | "border-top-style"
            | "border-top-width"
            | "border-width"
            | "bottom"
            | "box-shadow"
            | "box-sizing"
            | "break-after"
            | "break-before"
            | "break-inside"
            | "caption-side"
            | "caret-color"
            | "clear"
            | "clip"
            | "clip-path"
            | "color"
            | "column-count"
            | "column-fill"
            | "column-gap"
            | "column-rule"
            | "column-rule-color"
            | "column-rule-style"
            | "column-rule-width"
            | "column-span"
            | "column-width"
            | "columns"
            | "contain"
            | "content-visibility"
            | "counter-increment"
            | "counter-reset"
            | "cursor"
            | "direction"
            | "display"
            | "empty-cells"
            | "filter"
            | "flex"
            | "flex-basis"
            | "flex-direction"
            | "flex-flow"
            | "flex-grow"
            | "flex-shrink"
            | "flex-wrap"
            | "float"
            | "font"
            | "font-family"
            | "font-feature-settings"
            | "font-kerning"
            | "font-optical-sizing"
            | "font-size"
            | "font-size-adjust"
            | "font-stretch"
            | "font-style"
            | "font-variant"
            | "font-variant-caps"
            | "font-weight"
            | "gap"
            | "grid"
            | "grid-area"
            | "grid-auto-columns"
            | "grid-auto-flow"
            | "grid-auto-rows"
            | "grid-column"
            | "grid-column-end"
            | "grid-column-gap"
            | "grid-column-start"
            | "grid-gap"
            | "grid-row"
            | "grid-row-end"
            | "grid-row-gap"
            | "grid-row-start"
            | "grid-template"
            | "grid-template-areas"
            | "grid-template-columns"
            | "grid-template-rows"
            | "height"
            | "hyphens"
            | "inset"
            | "inset-block"
            | "inset-block-end"
            | "inset-block-start"
            | "inset-inline"
            | "inset-inline-end"
            | "inset-inline-start"
            | "isolation"
            | "justify-content"
            | "justify-items"
            | "justify-self"
            | "left"
            | "letter-spacing"
            | "line-break"
            | "line-height"
            | "list-style"
            | "list-style-image"
            | "list-style-position"
            | "list-style-type"
            | "margin"
            | "margin-block"
            | "margin-block-end"
            | "margin-block-start"
            | "margin-bottom"
            | "margin-inline"
            | "margin-inline-end"
            | "margin-inline-start"
            | "margin-left"
            | "margin-right"
            | "margin-top"
            | "mask"
            | "mask-border"
            | "mask-border-mode"
            | "mask-border-outset"
            | "mask-border-repeat"
            | "mask-border-slice"
            | "mask-border-source"
            | "mask-border-width"
            | "mask-clip"
            | "mask-composite"
            | "mask-image"
            | "mask-mode"
            | "mask-origin"
            | "mask-position"
            | "mask-repeat"
            | "mask-size"
            | "mask-type"
            | "max-block-size"
            | "max-height"
            | "max-inline-size"
            | "max-width"
            | "min-block-size"
            | "min-height"
            | "min-inline-size"
            | "min-width"
            | "mix-blend-mode"
            | "object-fit"
            | "object-position"
            | "offset"
            | "offset-anchor"
            | "offset-distance"
            | "offset-path"
            | "offset-position"
            | "offset-rotate"
            | "opacity"
            | "order"
            | "outline"
            | "outline-color"
            | "outline-offset"
            | "outline-style"
            | "outline-width"
            | "overflow"
            | "overflow-anchor"
            | "overflow-wrap"
            | "overflow-x"
            | "overflow-y"
            | "overscroll-behavior"
            | "overscroll-behavior-block"
            | "overscroll-behavior-inline"
            | "overscroll-behavior-x"
            | "overscroll-behavior-y"
            | "padding"
            | "padding-block"
            | "padding-block-end"
            | "padding-block-start"
            | "padding-bottom"
            | "padding-inline"
            | "padding-inline-end"
            | "padding-inline-start"
            | "padding-left"
            | "padding-right"
            | "padding-top"
            | "perspective"
            | "perspective-origin"
            | "place-content"
            | "place-items"
            | "place-self"
            | "pointer-events"
            | "position"
            | "quotes"
            | "resize"
            | "right"
            | "rotate"
            | "row-gap"
            | "scale"
            | "scroll-behavior"
            | "scroll-margin"
            | "scroll-margin-block"
            | "scroll-margin-block-end"
            | "scroll-margin-block-start"
            | "scroll-margin-bottom"
            | "scroll-margin-inline"
            | "scroll-margin-inline-end"
            | "scroll-margin-inline-start"
            | "scroll-margin-left"
            | "scroll-margin-right"
            | "scroll-margin-top"
            | "scroll-padding"
            | "scroll-padding-block"
            | "scroll-padding-block-end"
            | "scroll-padding-block-start"
            | "scroll-padding-bottom"
            | "scroll-padding-inline"
            | "scroll-padding-inline-end"
            | "scroll-padding-inline-start"
            | "scroll-padding-left"
            | "scroll-padding-right"
            | "scroll-padding-top"
            | "scroll-snap-align"
            | "scroll-snap-stop"
            | "scroll-snap-type"
            | "shape-image-threshold"
            | "shape-margin"
            | "shape-outside"
            | "tab-size"
            | "table-layout"
            | "text-align"
            | "text-align-last"
            | "text-combine-upright"
            | "text-decoration"
            | "text-decoration-color"
            | "text-decoration-line"
            | "text-decoration-style"
            | "text-decoration-thickness"
            | "text-indent"
            | "text-justify"
            | "text-orientation"
            | "text-overflow"
            | "text-rendering"
            | "text-shadow"
            | "text-transform"
            | "text-underline-offset"
            | "top"
            | "touch-action"
            | "transform"
            | "transform-origin"
            | "transform-style"
            | "transition"
            | "transition-delay"
            | "transition-duration"
            | "transition-property"
            | "transition-timing-function"
            | "translate"
            | "unicode-bidi"
            | "user-select"
            | "vertical-align"
            | "visibility"
            | "white-space"
            | "widows"
            | "width"
            | "will-change"
            | "word-break"
            | "word-spacing"
            | "word-wrap"
            | "writing-mode"
            | "z-index"
    )
}
