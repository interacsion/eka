use super::PublishArgs;
use clap::Parser;
use gix::{discover, remote::find::existing, ThreadSafeRepository};
use thiserror::Error;

#[derive(Error, Debug)]
enum GitError {
    #[error(transparent)]
    Discover(#[from] discover::Error),
    #[error(transparent)]
    RemotNotFound(#[from] existing::Error),
}

#[derive(Parser)]
#[command(next_help_heading = "Git Options")]
pub struct GitArgs {
    /// The repositories remote to publish the atom(s) to
    #[arg(long, default_value = "origin")]
    pub remote: String,
    /// The ref to publish the atom(s) from
    #[arg(long, default_value = "HEAD")]
    pub r#ref: String,
}

pub async fn run(_repo: ThreadSafeRepository, _args: PublishArgs) -> anyhow::Result<()> {
    Ok(())
}
