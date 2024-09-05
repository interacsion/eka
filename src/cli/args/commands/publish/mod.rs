#[cfg(feature = "git")]
mod git;

use crate::cli::vcs::{self, Vcs};

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(arg_required_else_help = true)]
pub struct PublishArgs {
    /// Publish all the atoms in and under the current working directory
    #[arg(long, short, conflicts_with = "path")]
    recursive: bool,

    /// Path(s) to the atom(s) to publish
    #[arg(required_unless_present = "recursive")]
    path: Vec<PathBuf>,

    #[command(flatten)]
    pub vcs_args: VcsArgs,
}

#[derive(Parser)]
pub struct VcsArgs {
    #[command(flatten)]
    #[cfg(feature = "git")]
    pub git: git::GitArgs,
}

pub async fn run(_args: PublishArgs) -> anyhow::Result<()> {
    let vcs = vcs::detect()?;
    match vcs {
        #[cfg(feature = "git")]
        Vcs::Git(path) => git::run(path).await?,
    }
    Ok(())
}
