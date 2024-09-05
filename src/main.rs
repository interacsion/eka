use clap::Parser;
use eka::cli::{self, Args};
use std::process::ExitCode;

#[tokio::main]
#[tracing::instrument]
async fn main() -> ExitCode {
    let args = Args::parse();
    let (v, q) = (args.verbosity, args.quiet);

    cli::init_logger(v, q);

    if let Err(e) = cli::run(args).await {
        if v > 0 && !q {
            tracing::error!("{:?}", e);
        } else {
            tracing::error!("{}", e);
        }
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
