pub mod commands;
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
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
    #[arg(
        short,
        long,
        action = clap::ArgAction::Count,
        global = true,
        help = "Increase logging verbosity",
        verbatim_doc_comment
    )]
    pub verbosity: u8,

    #[command(subcommand)]
    command: commands::Commands,
}
