mod commands;
mod logging;
mod vcs;

pub use commands::run;
pub use logging::init_logger;

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(flatten)]
    pub log: LogArgs,

    #[command(subcommand)]
    command: commands::Commands,
}

#[derive(Parser, Clone, Copy, Debug)]
#[command(next_help_heading = "Log Options")]
pub struct LogArgs {
    /// Set the level of verbosity
    ///
    /// This flag can be used multiple times to increase verbosity:
    ///   -v    for INFO level
    ///   -vv   for DEBUG level
    ///   -vvv  for TRACE level
    ///
    /// If not specified, defaults to WARN level.
    ///
    /// Alternatively, set the `RUST_LOG` environment variable
    /// (e.g., `RUST_LOG=info`), which takes precedence over this flag.
    ///
    /// Note: This flag is silently ignored when `--quiet` is also set.
    #[arg(
        short,
        long,
        action = clap::ArgAction::Count,
        global = true,
        help = "Increase logging verbosity",
        verbatim_doc_comment
    )]
    verbosity: u8,

    /// Suppress all output except errors
    ///
    /// This flag overrides any verbosity settings and sets the log
    /// level to ERROR. It takes precedence over both the `--verbosity`
    // flag and the `RUST_LOG` environment variable.
    ///
    /// Use this flag when you want minimal output from the application,
    /// typically in non-interactive or automated environments.
    #[arg(short, long, global = true, verbatim_doc_comment)]
    quiet: bool,
}
