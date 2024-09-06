use clap::Parser;
use eka::cli::{self, Args};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    let Args { log, .. } = args;

    cli::init_logger(log);

    if let Err(err) = cli::run(args).await {
        if log.verbosity > 1 && !log.quiet {
            // only print backtraces on debug or above
            tracing::error!("{:?}", err);
        } else {
            tracing::error!("{}", err);
        }
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
