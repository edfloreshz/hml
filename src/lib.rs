pub mod ast;
pub mod cli;
pub mod codegen;
pub mod diagnostics;
pub mod lexer;
pub mod lsp;
pub mod parser;

use std::fs;
use std::path::{Path, PathBuf};

pub use ast::{Attribute, Document, ElementNode, Node, Property, Span, TextNode, Value};
pub use codegen::{CodegenOutput, GeneratedClass};
pub use diagnostics::{Diagnostic, Diagnostics, Severity, SourceLocation};
pub use lexer::{LexedOutput, Lexer, Token, TokenKind};
pub use parser::{ParseResult, Parser};

#[derive(Debug, Clone)]
pub struct CompileResult {
    pub document: Option<Document>,
    pub output: Option<CodegenOutput>,
    pub diagnostics: Diagnostics,
}

impl CompileResult {
    pub fn is_success(&self) -> bool {
        !self.diagnostics.has_errors() && self.output.is_some()
    }

    pub fn html(&self) -> Option<&str> {
        self.output.as_ref().map(|output| output.html.as_str())
    }

    pub fn css(&self) -> Option<&str> {
        self.output.as_ref().map(|output| output.css.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct WrittenFile {
    pub source_path: PathBuf,
    pub html_path: PathBuf,
    pub css_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CompileDirectoryResult {
    pub files: Vec<WrittenFile>,
    pub diagnostics: Diagnostics,
}

impl CompileDirectoryResult {
    pub fn is_success(&self) -> bool {
        !self.diagnostics.has_errors()
    }

    pub fn files_written(&self) -> usize {
        self.files.len()
    }
}

pub fn compile(source: &str, file_name: impl Into<String>) -> CompileResult {
    let file_name = file_name.into();

    let lexed = Lexer::new(source, file_name.clone()).lex();
    let mut diagnostics = lexed.diagnostics;

    let mut parser = Parser::new(lexed.tokens, file_name.clone());
    let parsed = parser.parse();
    diagnostics.extend(parsed.diagnostics);

    if diagnostics.has_errors() {
        return CompileResult {
            document: None,
            output: None,
            diagnostics,
        };
    }

    let document = parsed.document;
    let output = codegen::generate(&document, file_name, &mut diagnostics);

    CompileResult {
        document: Some(document),
        output: Some(output),
        diagnostics,
    }
}

pub fn compile_file(path: impl AsRef<Path>) -> Result<CompileResult, std::io::Error> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)?;
    Ok(compile(&source, path.display().to_string()))
}

pub fn compile_to_files(
    input_path: impl AsRef<Path>,
    out_dir: impl AsRef<Path>,
) -> Result<CompileResult, std::io::Error> {
    let input_path = input_path.as_ref();
    let out_dir = out_dir.as_ref();

    let result = compile_file(input_path)?;

    if result.is_success() {
        let output = result.output.as_ref().expect("checked is_success");
        let html_path = html_output_path_single(input_path, out_dir);
        let css_path = css_output_path_single(input_path, out_dir);

        ensure_parent_dir(&html_path)?;
        ensure_parent_dir(&css_path)?;

        let linked_html = inject_stylesheet_link(&output.html, &css_path);
        fs::write(&html_path, linked_html)?;
        fs::write(&css_path, &output.css)?;
    }

    Ok(result)
}

pub fn compile_directory(
    input_dir: impl AsRef<Path>,
    out_dir: impl AsRef<Path>,
) -> Result<CompileDirectoryResult, std::io::Error> {
    let input_dir = input_dir.as_ref();
    let out_dir = out_dir.as_ref();

    let mut files = Vec::new();
    let mut diagnostics = Diagnostics::new();

    compile_directory_into(input_dir, input_dir, out_dir, &mut files, &mut diagnostics)?;

    Ok(CompileDirectoryResult { files, diagnostics })
}

pub fn compile_path_to_dir(
    input_path: impl AsRef<Path>,
    out_dir: impl AsRef<Path>,
) -> Result<CompileDirectoryResult, std::io::Error> {
    let input_path = input_path.as_ref();
    let out_dir = out_dir.as_ref();

    if input_path.is_dir() {
        return compile_directory(input_path, out_dir);
    }

    let result = compile_file(input_path)?;
    let mut files = Vec::new();
    let diagnostics = result.diagnostics.clone();

    if let Some(output) = result.output {
        if !diagnostics.has_errors() {
            let html_path = html_output_path_single(input_path, out_dir);
            let css_path = css_output_path_single(input_path, out_dir);

            ensure_parent_dir(&html_path)?;
            ensure_parent_dir(&css_path)?;

            let linked_html = inject_stylesheet_link(&output.html, &css_path);
            fs::write(&html_path, linked_html)?;
            fs::write(&css_path, output.css)?;

            files.push(WrittenFile {
                source_path: input_path.to_path_buf(),
                html_path,
                css_path,
            });
        }
    }

    Ok(CompileDirectoryResult { files, diagnostics })
}

pub fn format_diagnostic(diagnostic: &Diagnostic) -> String {
    diagnostic.render()
}

fn compile_directory_into(
    root_input_dir: &Path,
    current_dir: &Path,
    out_dir: &Path,
    files: &mut Vec<WrittenFile>,
    diagnostics: &mut Diagnostics,
) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            compile_directory_into(root_input_dir, &path, out_dir, files, diagnostics)?;
            continue;
        }

        if !is_hml_file(&path) {
            continue;
        }

        let result = compile_file(&path)?;
        diagnostics.extend(result.diagnostics.clone());

        if let Some(output) = result.output {
            if !result.diagnostics.has_errors() {
                let relative = relative_output_stem(root_input_dir, &path);
                let html_path = out_dir.join(&relative).with_extension("html");
                let css_path = out_dir.join(&relative).with_extension("css");

                ensure_parent_dir(&html_path)?;
                ensure_parent_dir(&css_path)?;

                let linked_html = inject_stylesheet_link(&output.html, &css_path);
                fs::write(&html_path, linked_html)?;
                fs::write(&css_path, output.css)?;

                files.push(WrittenFile {
                    source_path: path.clone(),
                    html_path,
                    css_path,
                });
            }
        }
    }

    Ok(())
}

fn html_output_path_single(input_path: &Path, out_dir: &Path) -> PathBuf {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");

    out_dir.join(format!("{stem}.html"))
}

fn css_output_path_single(input_path: &Path, out_dir: &Path) -> PathBuf {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");

    out_dir.join(format!("{stem}.css"))
}

fn relative_output_stem(root_input_dir: &Path, input_file: &Path) -> PathBuf {
    let relative = input_file
        .strip_prefix(root_input_dir)
        .unwrap_or(input_file);
    let mut output = relative.to_path_buf();
    output.set_extension("");
    output
}

fn ensure_parent_dir(path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    Ok(())
}

fn inject_stylesheet_link(html: &str, css_path: &Path) -> String {
    let stylesheet_name = css_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("styles.css");

    let link_tag = format!("<link rel=\"stylesheet\" href=\"{}\">", stylesheet_name);

    if let Some(style_index) = html.find("<style") {
        if html[..style_index].contains("<head>") {
            let insert_at = style_index;
            let mut linked = String::new();
            linked.push_str(&html[..insert_at]);
            linked.push_str("\t");
            linked.push_str(&link_tag);
            linked.push('\n');
            linked.push_str("\t\t");
            linked.push_str(&html[insert_at..]);
            return linked;
        }
    }

    if let Some(head_index) = html.find("</head>") {
        let mut linked = String::new();
        linked.push_str(&html[..head_index]);
        linked.push_str("\t");
        linked.push_str(&link_tag);
        linked.push('\n');
        linked.push('\t');
        linked.push_str(&html[head_index..]);
        linked
    } else {
        let mut linked = String::new();
        linked.push_str(&link_tag);
        linked.push('\n');
        linked.push_str(html);
        linked
    }
}

fn is_hml_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("hml"))
        .unwrap_or(false)
}
