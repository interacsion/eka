use crate::cli::store::Detected;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[command(flatten)]
    #[cfg(feature = "git")]
    git: GitArgs,
}

#[derive(Parser, Debug)]
#[command(next_help_heading = "Git Options")]
struct GitArgs {
    /// The target remote to initialize
    #[arg(long, short = 't', default_value_t = git::default_remote().to_owned(), name = "TARGET")]
    remote: String,
}

use atom::store::git;
pub(super) fn run(store: Option<Detected>, args: Args) -> Result<(), git::Error> {
    use atom::store::Init;
    if let Some(store) = store {
        match store {
            Detected::Git(repo) => repo.to_thread_local().ekala_init(args.git.remote)?,
        }
    }
    Ok(())
}
