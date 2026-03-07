use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum CliAction {
    Compile {
        input: PathBuf,
        out: PathBuf,
    },
    Watch {
        input: PathBuf,
        out: PathBuf,
    },
    Dev {
        input: PathBuf,
        out: PathBuf,
        host: String,
        port: u16,
    },
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
        Some("dev") => parse_dev_args(args),
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

    let out = parse_simple_out_args(&mut args, compile_usage)?;

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

    let out = parse_simple_out_args(&mut args, watch_usage)?;

    Ok(CliAction::Watch { input, out })
}

pub fn parse_dev_args<I>(mut args: I) -> Result<CliAction, String>
where
    I: Iterator<Item = String>,
{
    let mut input = PathBuf::from(".");
    let mut out = PathBuf::from("dist");
    let mut host = String::from("127.0.0.1");
    let mut port = 4000;

    if let Some(arg) = args.next() {
        if arg.starts_with("--") {
            match arg.as_str() {
                "--out" => {
                    out = args
                        .next()
                        .map(PathBuf::from)
                        .ok_or_else(|| dev_usage("missing output directory after --out"))?;
                }
                "--host" => {
                    host = args
                        .next()
                        .ok_or_else(|| dev_usage("missing host after --host"))?;
                }
                "--port" => {
                    let value = args
                        .next()
                        .ok_or_else(|| dev_usage("missing port after --port"))?;
                    port = value
                        .parse::<u16>()
                        .map_err(|_| dev_usage(&format!("invalid port '{value}'")))?;
                }
                extra => {
                    return Err(dev_usage(&format!("unexpected argument '{extra}'")));
                }
            }
        } else {
            input = PathBuf::from(arg);
        }
    }

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                out = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or_else(|| dev_usage("missing output directory after --out"))?;
            }
            "--host" => {
                host = args
                    .next()
                    .ok_or_else(|| dev_usage("missing host after --host"))?;
            }
            "--port" => {
                let value = args
                    .next()
                    .ok_or_else(|| dev_usage("missing port after --port"))?;
                port = value
                    .parse::<u16>()
                    .map_err(|_| dev_usage(&format!("invalid port '{value}'")))?;
            }
            extra if extra.starts_with("--") => {
                return Err(dev_usage(&format!("unexpected argument '{extra}'")));
            }
            extra => {
                return Err(dev_usage(&format!("unexpected argument '{extra}'")));
            }
        }
    }

    Ok(CliAction::Dev {
        input,
        out,
        host,
        port,
    })
}

pub fn help_text() -> String {
    format!(
        "{name} {version}
{about}

USAGE:
    {name} compile <INPUT> [--out <DIR>]
    {name} watch <INPUT> [--out <DIR>]
    {name} dev [INPUT] [--out <DIR>] [--host <HOST>] [--port <PORT>]

COMMANDS:
    compile    Compile a single .hml file or a directory of .hml files
    watch      Watch a single .hml file or directory and recompile on changes
    dev        Watch, serve, and live reload compiled output during development

OPTIONS:
    -h, --help       Print help
    -V, --version    Print version",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        about = env!("CARGO_PKG_DESCRIPTION")
    )
}

pub fn compile_usage(message: &str) -> String {
    format!("{message}\n\nUSAGE:\n    hml compile <INPUT> [--out <DIR>]")
}

pub fn watch_usage(message: &str) -> String {
    format!("{message}\n\nUSAGE:\n    hml watch <INPUT> [--out <DIR>]")
}

pub fn dev_usage(message: &str) -> String {
    format!(
        "{message}\n\nUSAGE:\n    hml dev [INPUT] [--out <DIR>] [--host <HOST>] [--port <PORT>]"
    )
}

fn parse_simple_out_args<I>(args: &mut I, usage: fn(&str) -> String) -> Result<PathBuf, String>
where
    I: Iterator<Item = String>,
{
    match args.next() {
        None => Ok(PathBuf::from("dist")),
        Some(flag) if flag == "--out" => {
            let out = args
                .next()
                .map(PathBuf::from)
                .ok_or_else(|| usage("missing output directory after --out"))?;

            if let Some(extra) = args.next() {
                return Err(usage(&format!("unexpected argument '{extra}'")));
            }

            Ok(out)
        }
        Some(flag) if flag.starts_with("--") => Err(usage("expected --out <DIR>")),
        Some(extra) => Err(usage(&format!("unexpected argument '{extra}'"))),
    }
}
