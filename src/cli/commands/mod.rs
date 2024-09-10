mod publish;
use super::Args;
use crate::cli::vcs;
use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Package and publish atoms directly within the project's VCS repository.
    ///
    /// This command implements a novel, decentralized publishing strategy:
    /// - Atoms are packaged into isolated VCS structures
    /// - Custom VCS references are created for efficient, path-based versioning
    ///
    /// The specific implementation varies by supported VCS:
    /// - Git: Uses orphan branches and custom refs for isolation and versioning
    ///
    /// This approach leverages existing VCS infrastructure for a self-contained,
    /// decentralized, and efficient atom registry system.
    #[command(verbatim_doc_comment)]
    Publish(publish::PublishArgs),
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    match args.command {
        Commands::Publish(args) => {
            let vcs = vcs::detect()?;
            publish::run(vcs, args).await?
        }
    }
    Ok(())
}
