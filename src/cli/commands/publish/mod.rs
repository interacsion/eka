#[cfg(feature = "git")]
mod git;

use crate::cli::vcs::Vcs;

use clap::Parser;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Parser)]
#[command(arg_required_else_help = true)]
pub(in super::super) struct PublishArgs {
    /// Publish all the atoms in and under the current working directory
    #[arg(long, short, conflicts_with = "path")]
    recursive: bool,

    /// Path(s) to the atom(s) to publish
    #[arg(required_unless_present = "recursive")]
    path: Vec<PathBuf>,
    #[command(flatten)]
    vcs: VcsArgs,
}

#[derive(Parser)]
struct VcsArgs {
    #[command(flatten)]
    #[cfg(feature = "git")]
    git: git::GitArgs,
}

pub(super) async fn run(vcs: Vcs, args: PublishArgs) -> Result<(), PublishError> {
    match vcs {
        #[cfg(feature = "git")]
        Vcs::Git(repo) => {
            git::run(repo, args).await?;
        }
    }
    Ok(())
}

#[derive(Error, Debug)]
pub(crate) enum PublishError {
    #[error(transparent)]
    #[cfg(feature = "git")]
    Git(#[from] git::error::GitError),
}
