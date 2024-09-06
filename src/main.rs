use clap::Parser;
use eka::cli::{self, Args};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    let Args { log, .. } = args;

    cli::init_logger(log);

    if (cli::run(args).await).is_err() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
