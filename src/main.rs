mod ansi;

use clap::Parser;
use eka::cli::{self, Args};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    let Args { log, .. } = args;

    let (_guard, ansi) = cli::init_global_subscriber(log);

    if let Err(e) = cli::run(args).await {
        tracing::error!(
            fatal = true,
            "{}FATAL{} {}",
            if ansi { ansi::MAGENTA } else { "" },
            if ansi { ansi::RESET } else { "" },
            e
        );
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
