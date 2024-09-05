use clap::Parser;
use eka::cli::{self, Args};

#[tokio::main]
#[tracing::instrument]
async fn main() {
    let args = Args::parse();
    let (v, q) = (args.verbosity, args.quiet);

    cli::init_logger(v, q);

    if let Err(e) = cli::run(args).await {
        if v > 0 && !q {
            tracing::error!("{:?}", e);
        } else {
            tracing::error!("{}", e);
        }
        std::process::exit(1);
    }
}
