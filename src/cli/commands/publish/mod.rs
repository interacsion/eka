#[cfg(feature = "git")]
mod git;

use crate::cli::store::Store;

use atom::publish::{
    error::{GitError, PublishError},
    Content,
};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
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

#[derive(Parser, Debug)]
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
                        let Content::Git(content) = atom.content();
                        tracing::info!(
                            atom.id = %atom.id().id(),
                            path = %content.path().display(),
                            "Atom successfully published"
                        );
                        tracing::debug!("published under: {}", content.ref_prefix());
                    }
                    Ok(Skipped(id)) => {
                        tracing::info!(atom.id = %id, "Skipping existing atom")
                    }
                    Err(e) => errors.push(e),
                }
            }

            for err in &errors {
                match err {
                    GitError::Invalid(e, path) => {
                        tracing::warn!(message = %err, path = %path.display(), message = format!("\n{}", e))
                    }
                    GitError::NotAFile(path) => {
                        tracing::warn!(message = %err, path = %path.display())
                    }
                    _ => tracing::warn!(message = %err),
                }
            }

            if !errors.is_empty() {
                return Err(PublishError::Git(GitError::Failed));
            }
        }
    }
    Ok(())
}
