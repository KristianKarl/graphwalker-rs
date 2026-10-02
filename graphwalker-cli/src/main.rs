mod commands;

use std::process;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "graphwalker",
    version = concat!(env!("CARGO_PKG_VERSION"), " (git ", env!("GRAPHWALKER_GIT_ID"), ")"),
    about = "Model-based testing tool"
)]
struct Cli {
    /// Set the logging verbosity
    #[arg(long, global = true, value_enum, default_value = "error")]
    log: LogLevel,

    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "lower")]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for tracing_subscriber::filter::LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Self::ERROR,
            LogLevel::Warn => Self::WARN,
            LogLevel::Info => Self::INFO,
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Trace => Self::TRACE,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Generate a test sequence offline
    Offline(commands::offline::Args),
    /// Start an online service (REST or WebSocket)
    Online(commands::online::Args),
    /// List all method names in the model(s)
    Methods(commands::methods::Args),
    /// List all requirements in the model(s)
    Requirements(commands::requirements::Args),
    /// Convert a model to another format
    Convert(commands::convert::Args),
    /// Generate source code from a model using a template
    Source(commands::source::Args),
    /// Check model(s) for issues
    Check(commands::check::Args),
}

fn main() {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing_subscriber::filter::LevelFilter::from(cli.log))
        .with_target(true)
        .init();

    let command_name = match &cli.command {
        Command::Offline(_) => "offline",
        Command::Online(_) => "online",
        Command::Methods(_) => "methods",
        Command::Requirements(_) => "requirements",
        Command::Convert(_) => "convert",
        Command::Source(_) => "source",
        Command::Check(_) => "check",
    };
    tracing::info!(command = command_name, "running CLI command");

    let result = match cli.command {
        Command::Offline(args) => commands::offline::run(args),
        Command::Online(args) => commands::online::run(args),
        Command::Methods(args) => commands::methods::run(args),
        Command::Requirements(args) => commands::requirements::run(args),
        Command::Convert(args) => commands::convert::run(args),
        Command::Source(args) => commands::source::run(args),
        Command::Check(args) => commands::check::run(args),
    };

    if let Err(e) = result {
        tracing::error!(error = %e, "CLI command failed");
        eprintln!("{}", e);
        process::exit(1);
    }
}
