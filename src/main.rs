mod cli;
use clap::Parser;
use cli::Args;

#[tokio::main]
#[tracing::instrument]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    cli::init_logger(args.verbosity);
    cli::run(args)?;

    Ok(())
}
