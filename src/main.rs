use std::{env, path::PathBuf, process};

use hml::{compile_path_to_dir, format_diagnostic};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    match hml::cli::parse_args(env::args().skip(1))? {
        hml::cli::CliAction::Compile { input, out } => run_compile(input, out),
        hml::cli::CliAction::Help => {
            println!("{}", hml::cli::help_text());
            Ok(())
        }
        hml::cli::CliAction::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}

fn run_compile(input: PathBuf, out_dir: PathBuf) -> Result<(), String> {
    let result = compile_path_to_dir(&input, &out_dir)
        .map_err(|error| format!("failed to compile '{}': {}", input.display(), error))?;

    for diagnostic in result.diagnostics.iter() {
        eprintln!("{}", format_diagnostic(diagnostic));
    }

    if result.diagnostics.has_errors() {
        return Err("compilation failed".to_string());
    }

    println!(
        "Compiled {} file(s) to {}",
        result.files_written(),
        out_dir.display()
    );

    Ok(())
}
