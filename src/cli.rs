use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum CliAction {
    Compile { input: PathBuf, out: PathBuf },
    Watch { input: PathBuf, out: PathBuf },
    Help,
    Version,
}

pub fn parse_args<I>(args: I) -> Result<CliAction, String>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();

    match args.next().as_deref() {
        Some("compile") => parse_compile_args(args),
        Some("watch") => parse_watch_args(args),
        Some("--help") | Some("-h") => Ok(CliAction::Help),
        Some("--version") | Some("-V") => Ok(CliAction::Version),
        Some(command) => Err(format!("unknown command '{command}'\n\n{}", help_text())),
        None => Ok(CliAction::Help),
    }
}

pub fn parse_compile_args<I>(mut args: I) -> Result<CliAction, String>
where
    I: Iterator<Item = String>,
{
    let input = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| compile_usage("missing input path"))?;

    let flag = args
        .next()
        .ok_or_else(|| compile_usage("missing required --out <DIR>"))?;

    if flag != "--out" {
        return Err(compile_usage("expected --out <DIR>"));
    }

    let out = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| compile_usage("missing output directory after --out"))?;

    if let Some(extra) = args.next() {
        return Err(compile_usage(&format!("unexpected argument '{extra}'")));
    }

    Ok(CliAction::Compile { input, out })
}

pub fn parse_watch_args<I>(mut args: I) -> Result<CliAction, String>
where
    I: Iterator<Item = String>,
{
    let input = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| watch_usage("missing input path"))?;

    let flag = args
        .next()
        .ok_or_else(|| watch_usage("missing required --out <DIR>"))?;

    if flag != "--out" {
        return Err(watch_usage("expected --out <DIR>"));
    }

    let out = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| watch_usage("missing output directory after --out"))?;

    if let Some(extra) = args.next() {
        return Err(watch_usage(&format!("unexpected argument '{extra}'")));
    }

    Ok(CliAction::Watch { input, out })
}

pub fn help_text() -> String {
    format!(
        "{name} {version}
{about}

USAGE:
    {name} compile <INPUT> --out <DIR>
    {name} watch <INPUT> --out <DIR>

COMMANDS:
    compile    Compile a single .hml file or a directory of .hml files
    watch      Watch a single .hml file or directory and recompile on changes

OPTIONS:
    -h, --help       Print help
    -V, --version    Print version",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        about = env!("CARGO_PKG_DESCRIPTION")
    )
}

pub fn compile_usage(message: &str) -> String {
    format!("{message}\n\nUSAGE:\n    hml compile <INPUT> --out <DIR>")
}

pub fn watch_usage(message: &str) -> String {
    format!("{message}\n\nUSAGE:\n    hml watch <INPUT> --out <DIR>")
}
