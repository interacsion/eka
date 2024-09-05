use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(next_help_heading = "Git Options")]
pub struct GitArgs {
    /// The remote to publish the atom(s) to
    #[arg(long, default_value = "origin")]
    pub remote: String,
}

pub async fn run(_path: PathBuf) -> anyhow::Result<()> {
    Ok(())
}
