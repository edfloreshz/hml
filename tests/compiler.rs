use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use hml::{compile, compile_directory, compile_file, compile_path_to_dir, compile_to_files};

fn test_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();

    let dir = std::env::temp_dir().join(format!("hml-compiler-{name}-{unique}"));
    fs::create_dir_all(&dir).expect("should create temporary test directory");
    dir
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("should create parent directories");
    }

    fs::write(path, contents).expect("should write test file");
}

#[test]
fn compile_generates_html_and_css_for_valid_source() {
    let result = compile(
        r#"
Box {
  color: #111111

  Paragraph {
    content: "Hello compiler"
  }
}
"#,
        "inline.hml",
    );

    assert!(result.is_success());
    assert!(!result.diagnostics.has_errors());

    let html = result.html().expect("expected generated html");
    let css = result.css().expect("expected generated css");

    assert!(html.contains("Hello compiler"));
    assert!(!css.is_empty());
}

#[test]
fn compile_returns_diagnostics_for_invalid_source() {
    let result = compile(
        r#"
div {
  color:
}
"#,
        "broken.hml",
    );

    assert!(!result.is_success());
    assert!(result.diagnostics.has_errors());
    assert!(result.html().is_none());
    assert!(result.css().is_none());
}

#[test]
fn compile_file_reads_source_from_disk() {
    let dir = test_dir("compile-file");
    let input = dir.join("page.hml");

    write_file(
        &input,
        r#"
Box {
  Paragraph {
    content: "Hello from file"
  }
}
"#,
    );

    let result = compile_file(&input).expect("should compile file from disk");

    assert!(result.is_success());

    let html = result.html().expect("expected generated html");
    assert!(html.contains("Hello from file"));
}

#[test]
fn compile_to_files_writes_html_and_css_outputs() {
    let dir = test_dir("compile-to-files");
    let input = dir.join("example.hml");
    let out_dir = dir.join("dist");

    write_file(
        &input,
        r#"
Box {
  color: #222222

  Paragraph {
    content: "Hello output files"
  }
}
"#,
    );

    let result = compile_to_files(&input, &out_dir).expect("should compile to files");

    assert!(result.is_success());

    let html_path = out_dir.join("example.html");
    let css_path = out_dir.join("example.css");

    assert!(html_path.exists());
    assert!(css_path.exists());

    let html = fs::read_to_string(&html_path).expect("should read generated html");
    let css = fs::read_to_string(&css_path).expect("should read generated css");

    assert!(html.contains("Hello output files"));
    assert!(html.contains(r#"rel="stylesheet""#));
    assert!(!css.is_empty());
}

#[test]
fn compile_to_files_skips_writing_outputs_when_compilation_fails() {
    let dir = test_dir("compile-to-files-error");
    let input = dir.join("broken.hml");
    let out_dir = dir.join("dist");

    write_file(
        &input,
        r#"
div {
  color:
}
"#,
    );

    let result = compile_to_files(&input, &out_dir).expect("should return diagnostics");

    assert!(!result.is_success());
    assert!(result.diagnostics.has_errors());
    assert!(!out_dir.join("broken.html").exists());
    assert!(!out_dir.join("broken.css").exists());
}

#[test]
fn compile_directory_writes_outputs_for_all_hml_files_recursively() {
    let dir = test_dir("compile-directory");
    let input_dir = dir.join("input");
    let out_dir = dir.join("dist");

    write_file(
        &input_dir.join("index.hml"),
        r#"
Box {
  Paragraph {
    content: "Root file"
  }
}
"#,
    );

    write_file(
        &input_dir.join("nested/about.hml"),
        r#"
Box {
  Paragraph {
    content: "Nested file"
  }
}
"#,
    );

    write_file(&input_dir.join("nested/ignore.txt"), "not hml");

    let result =
        compile_directory(&input_dir, &out_dir).expect("should compile directory recursively");

    assert!(result.is_success());
    assert_eq!(result.files_written(), 2);

    assert!(out_dir.join("index.html").exists());
    assert!(out_dir.join("index.css").exists());
    assert!(out_dir.join("nested/about.html").exists());
    assert!(out_dir.join("nested/about.css").exists());
    assert!(!out_dir.join("nested/ignore.html").exists());
}

#[test]
fn compile_path_to_dir_handles_single_file_input() {
    let dir = test_dir("compile-path-file");
    let input = dir.join("single.hml");
    let out_dir = dir.join("dist");

    write_file(
        &input,
        r#"
Box {
  Paragraph {
    content: "Single path compile"
  }
}
"#,
    );

    let result = compile_path_to_dir(&input, &out_dir).expect("should compile single input path");

    assert!(result.is_success());
    assert_eq!(result.files_written(), 1);
    assert!(out_dir.join("single.html").exists());
    assert!(out_dir.join("single.css").exists());
}

#[test]
fn compile_path_to_dir_handles_directory_input() {
    let dir = test_dir("compile-path-directory");
    let input_dir = dir.join("pages");
    let out_dir = dir.join("dist");

    write_file(
        &input_dir.join("home.hml"),
        r#"
Box {
  Paragraph {
    content: "Home"
  }
}
"#,
    );

    write_file(
        &input_dir.join("blog/post.hml"),
        r#"
Box {
  Paragraph {
    content: "Post"
  }
}
"#,
    );

    let result =
        compile_path_to_dir(&input_dir, &out_dir).expect("should compile directory input path");

    assert!(result.is_success());
    assert_eq!(result.files_written(), 2);
    assert!(out_dir.join("home.html").exists());
    assert!(out_dir.join("home.css").exists());
    assert!(out_dir.join("blog/post.html").exists());
    assert!(out_dir.join("blog/post.css").exists());
}

#[test]
fn compile_directory_collects_diagnostics_and_skips_invalid_files() {
    let dir = test_dir("compile-directory-partial-failure");
    let input_dir = dir.join("input");
    let out_dir = dir.join("dist");

    write_file(
        &input_dir.join("good.hml"),
        r#"
Box {
  Paragraph {
    content: "Good"
  }
}
"#,
    );

    write_file(
        &input_dir.join("bad.hml"),
        r#"
div {
  color:
}
"#,
    );

    let result = compile_directory(&input_dir, &out_dir).expect("should compile directory");

    assert!(!result.is_success());
    assert!(result.diagnostics.has_errors());
    assert_eq!(result.files_written(), 1);

    assert!(out_dir.join("good.html").exists());
    assert!(out_dir.join("good.css").exists());
    assert!(!out_dir.join("bad.html").exists());
    assert!(!out_dir.join("bad.css").exists());
}
