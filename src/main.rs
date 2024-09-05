use clap::Parser;
use eka::cli::{self, Args};

#[tokio::main]
#[tracing::instrument]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    cli::init_logger(args.verbosity, args.quiet);

    cli::run(args).await
}
