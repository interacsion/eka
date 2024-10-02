mod init;
mod publish;

use super::Args;
use crate::cli::store;

use clap::Subcommand;

#[derive(Subcommand)]
pub(super) enum Commands {
    /// Package and publish atoms to the atom store.
    ///
    /// This command efficiently packages and publishes atoms using Git:
    ///
    /// - Creates isolated structures (orphan branches) for each atom
    /// - Uses custom Git refs for versioning and rapid, path-based fetching
    /// - Enables decentralized publishing while minimizing data transfer
    ///
    /// The atom store concept is designed to be extensible, allowing for
    /// future support of alternative storage backends as well.
    #[command(verbatim_doc_comment)]
    Publish(publish::PublishArgs),
    Init(init::Args),
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let store = store::detect();
    match args.command {
        Commands::Publish(args) => {
            publish::run(store.await?, args).await?;
        }

        Commands::Init(args) => init::run(store.await.ok(), args)?,
    }
    Ok(())
}
