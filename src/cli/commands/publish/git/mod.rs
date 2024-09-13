pub(super) mod error;

mod r#impl;

use super::PublishArgs;
use crate::cli::logging::{self, LogValue};

use clap::Parser;
use error::GitError;
use std::cell::RefCell;
use std::path::PathBuf;
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

pub(super) async fn run(repo: ThreadSafeRepository, args: PublishArgs) -> Result<(), GitError> {
    let repo = repo.to_thread_local();

    let context = PublishGitContext::set(&repo, args.vcs.git).await?;

    let atoms: Vec<()> = if args.recursive {
        todo!();
    } else {
        context.publish(args.path)
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

    fn publish<C>(&self, paths: C) -> Vec<()>
    where
        C: IntoIterator<Item = PathBuf>,
    {
        paths
            .into_iter()
            .filter_map(|path| {
                let atom_path = if matches!(path.extension(), Some(ext) if ext == "atom") {
                    &path
                } else {
                    &path.with_extension("atom")
                };
                self.repo
                    .work_dir()
                    .map_or_else(
                        || self.publish_atom(atom_path),
                        |rel_repo| self.publish_workdir_atom(rel_repo, atom_path),
                    )
                    .or_else(|| {
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
