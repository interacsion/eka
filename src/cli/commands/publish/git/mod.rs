pub(super) mod error;

mod r#impl;

use super::PublishArgs;
use crate::cli::logging::{self, LogValue};

use clap::Parser;
use error::GitError;
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::{sync::Mutex, task::JoinSet};

use gix::{Commit, Remote, Repository, ThreadSafeRepository, Tree};

#[derive(Parser, Debug)]
#[command(next_help_heading = "Git Options")]
pub(super) struct GitArgs {
    /// The target remote to publish the atom(s) to
    #[arg(long, short = 't', default_value = "origin", name = "TARGET")]
    remote: String,
    /// The revision to publish the atom(s) from
    ///
    /// Specifies a revision using Git's extended SHA-1 syntax.
    /// This can be a commit hash, branch name, tag, or a relative
    /// reference like HEAD~3 or master@{yesterday}.
    #[arg(
        long,
        short,
        default_value = "HEAD",
        verbatim_doc_comment,
        name = "REVSPEC"
    )]
    spec: String,
}

pub(super) async fn run(repo: ThreadSafeRepository, args: PublishArgs) -> Result<(), GitError> {
    let repo = repo.to_thread_local();

    let context = PublishGitContext::set(&repo, args.store.git).await?;

    let atoms: Vec<()> = if args.recursive {
        todo!();
    } else {
        // filter redundant paths
        let paths: HashSet<PathBuf> = args.path.into_iter().collect();
        context.publish(paths)
    };

    if atoms.is_empty() {
        return Err(logging::log_error(GitError::AllFailed));
    }

    let mut errors = Vec::new();
    context.await_pushes(&mut errors).await;

    if errors.is_empty() {
        Ok(())
    } else {
        Err(logging::log_error(GitError::SomePushFailed))
    }
}

trait ExtendRepo {
    /// Normalizes a given path to be relative to the repository root.
    ///
    /// This function takes a path (relative or absolute) and attempts to normalize it
    /// relative to the repository root, based on the current working directory within
    /// the repository's file system.
    ///
    /// # Behavior:
    /// - For relative paths (e.g., "foo/bar" or "../foo"):
    ///   - Interpreted as relative to the current working directory within the repository.
    ///   - Computed relative to the repository root.
    ///
    /// - For absolute paths (e.g., "/foo/bar"):
    ///   - Treated as if the repository root is the filesystem root.
    ///   - The leading slash is ignored, and the path is considered relative to the repo root.
    ///
    /// # Arguments
    /// * `path` - A `&Path` representing the path to normalize.
    ///
    /// # Returns
    /// * `Some(Ok(PathBuf))` - A normalized path relative to the repository root.
    /// * `Some(Err(()))` - A special case to allow the caller to short-circuit if the resulting path escapes the repo root
    /// * `None` - If normalization fails. This can occur in scenarios such as:
    ///   - The function is called in a bare repository where the concept of a working directory doesn't apply.
    ///   - The current working directory is outside the repository.
    fn normalize(&self, path: &Path) -> Option<Result<PathBuf, ()>>;
}

#[derive(Debug)]
/// Holds the shared context needed for publishing atoms
struct PublishGitContext<'a> {
    /// Reference to the repository we are publish from
    repo: &'a Repository,
    /// The repository tree object for the given commit
    tree: Tree<'a>,
    /// The commit to publish from
    commit: Commit<'a>,
    /// The remote to publish to
    remote: Remote<'a>,
    /// a JoinSet of push tasks to avoid blocking on them
    push_tasks: RefCell<JoinSet<Result<Vec<u8>, GitError>>>,
}

impl<'a> PublishGitContext<'a> {
    async fn set(repo: &'a Repository, args: GitArgs) -> Result<Self, GitError> {
        let GitArgs {
            ref remote,
            ref spec,
        } = args;
        let remote = async { repo.find_remote(remote.as_str()).log_err() };

        let commit = async {
            repo.rev_parse_single(spec.as_str())
                .log_err()
                .map(|s| repo.find_commit(s).log_err())
        };

        // print both errors before returning one
        let (remote, commit) = tokio::join!(remote, commit);
        let (remote, commit) = (remote?, commit??);

        let tree = commit.tree().log_err()?;

        let push_tasks = RefCell::new(JoinSet::new());

        Ok(Self {
            repo,
            tree,
            commit,
            remote,
            push_tasks,
        })
    }

    /// Publishes atoms, attempting to normalize the given path(s) relative to the caller's location within the repository.
    ///
    /// This function processes a collection of paths, each representing an atom to be published. The publishing
    /// process includes path normalization, existence checks, and actual publishing attempts.
    ///
    /// # Path Normalization
    /// - First attempts to interpret each path as relative to the caller's current location inside the repository.
    /// - If normalization fails (e.g., in a bare repository), falls back to treating the path as already relative to the repo root.
    /// - The normalized path is used to search the Git history, not the file system.
    ///
    /// # Publishing Process
    /// For each path:
    /// 1. Normalizes the path (as described above).
    /// 2. Checks if the atom already exists in the repository.
    ///    - If it exists, the atom is skipped, and a log message is generated.
    /// 3. Attempts to publish the atom.
    ///    - If successful, the atom is added to the repository.
    ///    - If any error occurs during publishing, the atom is skipped, and an error is logged.
    ///
    /// # Error Handling
    /// - The function aims to process all provided paths, even if some fail.
    /// - Errors and skipped atoms are reported via logs but do not halt the overall process.
    /// - The function continues to the next atom after logging any errors or skip conditions.
    ///
    /// # Return Value
    /// Returns a vector of unit types (`Vec<()>`), primarily to indicate the number of paths processed.
    /// The caller should check the logs for detailed information on the success or failure of each atom.
    ///
    /// # Parameters
    /// - `paths`: A collection `C` of paths, each representing an atom to be published.
    ///
    /// # Note
    /// This function prioritizes completing the publishing attempt for all provided paths over
    /// stopping at the first error. Check the logs for a comprehensive report of the publishing process.
    fn publish<C>(&self, paths: C) -> Vec<()>
    where
        C: IntoIterator<Item = PathBuf>,
    {
        paths
            .into_iter()
            .filter_map(|path| {
                let path = match self.repo.normalize(&path.with_extension("atom")) {
                    Some(Ok(path)) => path,
                    None => path,
                    Some(Err(_)) => return None,
                };
                self.publish_atom(&path).or_else(|| {
                    tracing::warn!(message = "Skipping atom", path = %path.display());
                    None
                })
            })
            .collect()
    }

    async fn await_pushes(&self, errors: &mut Vec<GitError>) {
        let tasks = Mutex::new(self.push_tasks.borrow_mut());

        while let Some(task) = tasks.lock().await.join_next().await {
            match task {
                Ok(Ok(output)) => {
                    if !output.is_empty() {
                        tracing::info!(output = %String::from_utf8_lossy(&output));
                    }
                }
                Ok(Err(e)) => {
                    errors.push(logging::log_error(e));
                }
                Err(e) => {
                    errors.push(logging::log_error(GitError::JoinFailed(e)));
                }
            }
        }
    }
}

impl ExtendRepo for Repository {
    fn normalize(&self, path: &Path) -> Option<Result<PathBuf, ()>> {
        use path_clean::PathClean;
        use std::fs;

        let rel_repo_root = self.work_dir()?;
        let repo_root = fs::canonicalize(rel_repo_root).log_err().ok()?;
        let current = self.current_dir();
        let rel = current
            .join(path)
            .clean()
            .strip_prefix(&repo_root)
            .map(Path::to_path_buf);

        rel.or_else(|e| {
            // handle absolute paths as if they were relative to the repo root
            if !path.is_absolute() {
                return Err(e);
            }
            let cleaned = path.clean();
            // Preserve the platform-specific root
            let p = cleaned.strip_prefix(Path::new("/")).log_err()?;
            repo_root
                .join(p)
                .clean()
                .strip_prefix(&repo_root)
                .map(ToOwned::to_owned)
        })
        .map_err(|_| {
            tracing::warn!(
                message = "Ignoring path outside repo root",
                path = %path.display(),
            );
        })
        .into()
    }
}
