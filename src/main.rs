mod dev;

use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
    sync::mpsc,
    time::{Duration, Instant},
};

use hml::{CompileDirectoryResult, compile_path_to_dir, format_diagnostic};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

#[tokio::main]
async fn run() -> Result<(), String> {
    match hml::cli::parse_args(env::args().skip(1))? {
        hml::cli::CliAction::Compile { input, out } => run_compile(input, out),
        hml::cli::CliAction::Watch { input, out } => run_watch(input, out),
        hml::cli::CliAction::Dev {
            input,
            out,
            host,
            port,
        } => run_dev(input, out, host, port).await,
        hml::cli::CliAction::Lsp => run_lsp().await,
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

async fn run_lsp() -> Result<(), String> {
    use tokio::io::{stdin, stdout};
    use tower_lsp::LspService;
    use tower_lsp::Server;

    let (service, socket) = LspService::new(hml::lsp::HmlLanguageServer::new);
    Server::new(stdin(), stdout(), socket).serve(service).await;

    Ok(())
}

fn run_compile(input: PathBuf, out_dir: PathBuf) -> Result<(), String> {
    compile_once(&input, &out_dir).map(|_| ())
}

fn run_watch(input: PathBuf, out_dir: PathBuf) -> Result<(), String> {
    compile_once(&input, &out_dir).map(|_| ())?;

    println!(
        "Watching {} for changes. Press Ctrl+C to stop.",
        input.display()
    );

    watch_rebuild_loop(&input, &out_dir, |_| Ok(()))
}

async fn run_dev(input: PathBuf, out_dir: PathBuf, host: String, port: u16) -> Result<(), String> {
    let initial_result = compile_once(&input, &out_dir)?;
    dev::inject_live_reload(&initial_result)?;

    let (server, address) = dev::DevServer::start(out_dir.clone(), &host, port).await?;
    let dev_url = format!("http://{}", address);

    println!("[hml] dev server running at {dev_url}");
    println!("[hml] serving {}", out_dir.display());
    println!("[hml] watching {}", input.display());

    open_in_browser(&dev_url)?;

    watch_rebuild_loop(&input, &out_dir, move |result| {
        dev::inject_live_reload(result)?;
        server.notify_reload();
        println!("[hml] reloaded browser");
        Ok(())
    })
}

fn watch_rebuild_loop<F>(input: &Path, out_dir: &Path, mut on_success: F) -> Result<(), String>
where
    F: FnMut(&CompileDirectoryResult) -> Result<(), String>,
{
    let watch_target = watch_target(input);
    let recursive_mode = if input.is_dir() {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
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
        .watch(&watch_target, recursive_mode)
        .map_err(|error| format!("failed to watch '{}': {}", watch_target.display(), error))?;

    let mut last_rebuild = Instant::now()
        .checked_sub(Duration::from_secs(1))
        .unwrap_or_else(Instant::now);

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                if !event_should_trigger_rebuild(&event, input) {
                    continue;
                }

                let now = Instant::now();
                if now.duration_since(last_rebuild) < Duration::from_millis(75) {
                    continue;
                }
                last_rebuild = now;

                if let Some(path) = event.paths.first() {
                    println!("[hml] change detected: {}", path.display());
                } else {
                    println!("[hml] change detected");
                }

                match compile_once(input, out_dir) {
                    Ok(result) => {
                        on_success(&result)?;
                        println!("[hml] watching for changes...");
                    }
                    Err(error) => {
                        eprintln!("[hml] rebuild failed");
                        eprintln!("{error}");
                        eprintln!("[hml] waiting for the next change...");
                    }
                }
            }
            Ok(Err(error)) => eprintln!("watch error: {error}"),
            Err(error) => return Err(format!("watch channel closed: {error}")),
        }
    }
}

fn compile_once(input: &Path, out_dir: &Path) -> Result<CompileDirectoryResult, String> {
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

    Ok(result)
}

fn watch_target(input: &Path) -> PathBuf {
    if input.is_dir() {
        input.to_path_buf()
    } else {
        input
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }
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

fn open_in_browser(url: &str) -> Result<(), String> {
    let command = if cfg!(target_os = "macos") {
        ("open", vec![url])
    } else if cfg!(target_os = "windows") {
        ("cmd", vec!["/C", "start", url])
    } else {
        ("xdg-open", vec![url])
    };

    process::Command::new(command.0)
        .args(command.1)
        .spawn()
        .map_err(|error| format!("failed to open browser at '{url}': {error}"))?;

    Ok(())
}
