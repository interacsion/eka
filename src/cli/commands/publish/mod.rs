#[cfg(feature = "git")]
mod git;

use crate::cli::{
    logging::{self},
    store::Store,
};

use atom::publish::error::{GitError, PublishError};
use clap::Parser;
use std::path::PathBuf;

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
    store: StoreArgs,
}

#[derive(Parser)]
struct StoreArgs {
    #[command(flatten)]
    #[cfg(feature = "git")]
    git: git::GitArgs,
}

pub(super) async fn run(store: Store, args: PublishArgs) -> Result<(), PublishError> {
    // use atom::publish::Content;
    use Err as Skipped;
    use Ok as Published;
    match store {
        #[cfg(feature = "git")]
        Store::Git(repo) => {
            let (results, mut errors) = git::run(repo, args).await?;

            for res in results {
                match res {
                    Ok(Published(atom)) => {
                        // TODO: flesh out logging
                        // let Content::Git(content) = atom.content();
                        tracing::info!(atom.id = %atom.id().id(),  "Atom successfully published")
                    }
                    Ok(Skipped(id)) => {
                        tracing::warn!(atom.id = %id, "Skipping existing atom")
                    }
                    Err(e) => errors.push(e),
                }
            }

            for err in &errors {
                tracing::error!(%err);
            }

            if !errors.is_empty() {
                return Err(logging::log_error(PublishError::Git(GitError::Failed)));
            }
        }
    }
    Ok(())
}
