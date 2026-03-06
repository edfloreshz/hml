use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
    sync::mpsc,
    time::{Duration, Instant},
};

use hml::{compile_path_to_dir, format_diagnostic};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    match hml::cli::parse_args(env::args().skip(1))? {
        hml::cli::CliAction::Compile { input, out } => run_compile(input, out),
        hml::cli::CliAction::Watch { input, out } => run_watch(input, out),
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
    compile_once(&input, &out_dir)
}

fn run_watch(input: PathBuf, out_dir: PathBuf) -> Result<(), String> {
    compile_once(&input, &out_dir)?;

    let watch_target = if input.is_dir() {
        input.clone()
    } else {
        input
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |result| {
            let _ = tx.send(result);
        },
        Config::default(),
    )
    .map_err(|error| format!("failed to initialize file watcher: {error}"))?;

    watcher
        .watch(
            &watch_target,
            if input.is_dir() {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            },
        )
        .map_err(|error| format!("failed to watch '{}': {}", watch_target.display(), error))?;

    println!(
        "Watching {} for changes. Press Ctrl+C to stop.",
        input.display()
    );

    let mut last_rebuild = Instant::now()
        .checked_sub(Duration::from_secs(1))
        .unwrap_or_else(Instant::now);

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                if !event_should_trigger_rebuild(&event, &input) {
                    continue;
                }

                let now = Instant::now();
                if now.duration_since(last_rebuild) < Duration::from_millis(75) {
                    continue;
                }
                last_rebuild = now;

                println!("Change detected, rebuilding...");
                if let Err(error) = compile_once(&input, &out_dir) {
                    eprintln!("{error}");
                }
            }
            Ok(Err(error)) => eprintln!("watch error: {error}"),
            Err(error) => return Err(format!("watch channel closed: {error}")),
        }
    }
}

fn compile_once(input: &Path, out_dir: &Path) -> Result<(), String> {
    let result = compile_path_to_dir(input, out_dir)
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

fn event_should_trigger_rebuild(event: &Event, input: &Path) -> bool {
    if !matches!(
        event.kind,
        EventKind::Create(_)
            | EventKind::Modify(_)
            | EventKind::Remove(_)
            | EventKind::Any
            | EventKind::Other
    ) {
        return false;
    }

    event_affects_input(event, input)
}

fn event_affects_input(event: &Event, input: &Path) -> bool {
    if input.is_dir() {
        return event.paths.iter().any(|path| is_hml_path(path));
    }

    let input_file_name = input.file_name();

    event.paths.iter().any(|path| {
        path == input
            || normalize_path(path).as_deref() == normalize_path(input).as_deref()
            || (is_hml_path(path) && path.file_name() == input_file_name)
    })
}

fn normalize_path(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

fn is_hml_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("hml"))
        .unwrap_or(false)
}
