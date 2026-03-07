#[path = "../src/cli.rs"]
mod cli;

use std::path::PathBuf;

use cli::{CliAction, parse_args};

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn parses_help_when_no_args_are_provided() {
    let action = parse_args(Vec::<String>::new()).expect("expected help action");
    assert_eq!(action, CliAction::Help);
}

#[test]
fn parses_help_flag() {
    let action = parse_args(strings(&["--help"])).expect("expected help action");
    assert_eq!(action, CliAction::Help);
}

#[test]
fn parses_short_help_flag() {
    let action = parse_args(strings(&["-h"])).expect("expected help action");
    assert_eq!(action, CliAction::Help);
}

#[test]
fn parses_version_flag() {
    let action = parse_args(strings(&["--version"])).expect("expected version action");
    assert_eq!(action, CliAction::Version);
}

#[test]
fn parses_short_version_flag() {
    let action = parse_args(strings(&["-V"])).expect("expected version action");
    assert_eq!(action, CliAction::Version);
}

#[test]
fn parses_compile_command() {
    let action = parse_args(strings(&["compile", "input.hml"])).expect("expected compile action");

    assert_eq!(
        action,
        CliAction::Compile {
            input: PathBuf::from("input.hml"),
            out: PathBuf::from("dist"),
        }
    );
}

#[test]
fn parses_watch_command() {
    let action = parse_args(strings(&["watch", "input.hml"])).expect("expected watch action");

    assert_eq!(
        action,
        CliAction::Watch {
            input: PathBuf::from("input.hml"),
            out: PathBuf::from("dist"),
        }
    );
}

#[test]
fn parses_dev_command() {
    let action = parse_args(strings(&["dev", "input.hml"])).expect("expected dev action");

    assert_eq!(
        action,
        CliAction::Dev {
            input: PathBuf::from("input.hml"),
            out: PathBuf::from("dist"),
            host: "127.0.0.1".to_string(),
            port: 4000,
        }
    );
}

#[test]
fn parses_dev_command_with_all_options() {
    let action = parse_args(strings(&[
        "dev", "examples", "--out", "dist", "--host", "0.0.0.0", "--port", "4000",
    ]))
    .expect("expected dev action");

    assert_eq!(
        action,
        CliAction::Dev {
            input: PathBuf::from("examples"),
            out: PathBuf::from("dist"),
            host: "0.0.0.0".to_string(),
            port: 4000,
        }
    );
}

#[test]
fn rejects_compile_without_input() {
    let error = parse_args(strings(&["compile"])).expect_err("expected parse error");
    assert!(error.contains("missing input path"));
}

#[test]
fn parses_compile_command_with_explicit_out_flag() {
    let action = parse_args(strings(&["compile", "input.hml", "--out", "build"]))
        .expect("expected compile action");

    assert_eq!(
        action,
        CliAction::Compile {
            input: PathBuf::from("input.hml"),
            out: PathBuf::from("build"),
        }
    );
}

#[test]
fn rejects_compile_with_wrong_flag_position() {
    let error = parse_args(strings(&["compile", "--out", "dist", "input.hml"]))
        .expect_err("expected parse error");
    assert!(!error.trim().is_empty());
}

#[test]
fn rejects_compile_without_output_directory() {
    let error =
        parse_args(strings(&["compile", "input.hml", "--out"])).expect_err("expected parse error");
    assert!(error.contains("missing output directory after --out"));
}

#[test]
fn rejects_compile_with_extra_argument() {
    let error =
        parse_args(strings(&["compile", "input.hml", "extra"])).expect_err("expected parse error");
    assert!(error.contains("unexpected argument 'extra'"));
}

#[test]
fn rejects_watch_without_input() {
    let error = parse_args(strings(&["watch"])).expect_err("expected parse error");
    assert!(error.contains("missing input path"));
}

#[test]
fn rejects_dev_without_input() {
    let error = parse_args(strings(&["dev"])).expect_err("expected parse error");
    assert!(error.contains("missing input path"));
}

#[test]
fn parses_watch_command_with_explicit_out_flag() {
    let action = parse_args(strings(&["watch", "input.hml", "--out", "build"]))
        .expect("expected watch action");

    assert_eq!(
        action,
        CliAction::Watch {
            input: PathBuf::from("input.hml"),
            out: PathBuf::from("build"),
        }
    );
}

#[test]
fn parses_dev_command_with_explicit_out_flag() {
    let action =
        parse_args(strings(&["dev", "input.hml", "--out", "build"])).expect("expected dev action");

    assert_eq!(
        action,
        CliAction::Dev {
            input: PathBuf::from("input.hml"),
            out: PathBuf::from("build"),
            host: "127.0.0.1".to_string(),
            port: 4000,
        }
    );
}

#[test]
fn rejects_watch_with_wrong_flag_position() {
    let error = parse_args(strings(&["watch", "--out", "dist", "input.hml"]))
        .expect_err("expected parse error");
    assert!(!error.trim().is_empty());
}

#[test]
fn rejects_dev_with_wrong_flag_position() {
    let error = parse_args(strings(&["dev", "--out", "dist", "input.hml"]))
        .expect_err("expected parse error");
    assert!(!error.trim().is_empty());
}

#[test]
fn rejects_watch_without_output_directory() {
    let error =
        parse_args(strings(&["watch", "input.hml", "--out"])).expect_err("expected parse error");
    assert!(error.contains("missing output directory after --out"));
}

#[test]
fn rejects_dev_without_output_directory() {
    let error =
        parse_args(strings(&["dev", "input.hml", "--out"])).expect_err("expected parse error");
    assert!(error.contains("missing output directory after --out"));
}

#[test]
fn rejects_watch_with_extra_argument() {
    let error =
        parse_args(strings(&["watch", "input.hml", "extra"])).expect_err("expected parse error");
    assert!(error.contains("unexpected argument 'extra'"));
}

#[test]
fn rejects_dev_with_extra_argument() {
    let error = parse_args(strings(&["dev", "input.hml", "--out", "dist", "extra"]))
        .expect_err("expected parse error");
    assert!(error.contains("unexpected argument 'extra'"));
}

#[test]
fn rejects_unknown_command() {
    let error = parse_args(strings(&["wat"])).expect_err("expected parse error");
    assert!(error.contains("unknown command 'wat'"));
}

#[test]
fn parses_watch_command_with_file_input() {
    let action = parse_args(strings(&["watch", "examples/blog.hml", "--out", "dist"]))
        .expect("expected watch action");

    assert_eq!(
        action,
        CliAction::Watch {
            input: PathBuf::from("examples/blog.hml"),
            out: PathBuf::from("dist"),
        }
    );
}

#[test]
fn parses_watch_command_with_directory_input() {
    let action = parse_args(strings(&["watch", "examples", "--out", "dist/examples"]))
        .expect("expected watch action");

    assert_eq!(
        action,
        CliAction::Watch {
            input: PathBuf::from("examples"),
            out: PathBuf::from("dist/examples"),
        }
    );
}

#[test]
fn rejects_watch_command_missing_out_value_for_file_input() {
    let error = parse_args(strings(&["watch", "examples/blog.hml", "--out"]))
        .expect_err("expected parse error");
    assert!(error.contains("missing output directory after --out"));
}

#[test]
fn rejects_watch_command_extra_argument_for_directory_input() {
    let error =
        parse_args(strings(&["watch", "examples", "extra"])).expect_err("expected parse error");
    assert!(error.contains("unexpected argument 'extra'"));
}

#[test]
fn rejects_dev_with_invalid_port() {
    let error = parse_args(strings(&[
        "dev", "examples", "--out", "dist", "--port", "abc",
    ]))
    .expect_err("expected parse error");
    assert!(error.contains("invalid port"));
}

#[test]
fn rejects_dev_with_missing_host_value() {
    let error = parse_args(strings(&["dev", "examples", "--out", "dist", "--host"]))
        .expect_err("expected parse error");
    assert!(error.contains("missing host after --host"));
}

#[test]
fn rejects_dev_with_missing_port_value() {
    let error = parse_args(strings(&["dev", "examples", "--out", "dist", "--port"]))
        .expect_err("expected parse error");
    assert!(error.contains("missing port after --port"));
}
