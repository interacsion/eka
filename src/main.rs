mod ansi;

use clap::Parser;
use eka::cli::{self, Args};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    let Args { log, .. } = args;

    let _guard = cli::init_global_subscriber(log);

    if let Err(e) = cli::run(args).await {
        tracing::error!("{}FATAL{} {}", ansi::MAGENTA, ansi::RESET, e);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
