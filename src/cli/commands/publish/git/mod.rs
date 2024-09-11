pub(super) mod error;

mod r#impl;

use super::PublishArgs;
use crate::cli::logging::LogValue;

use clap::Parser;
use error::GitError;
use std::path::PathBuf;

use gix::{Commit, Remote, Repository, ThreadSafeRepository, Tree};

#[derive(Parser)]
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
// Define a struct to hold the context for publishing atoms
struct PublishGitContext<'a> {
    // Reference to the repository we are publish from
    repo: &'a Repository,
    // The repository tree object for the given commit
    tree: Tree<'a>,
    // The commit to publish from
    commit: Commit<'a>,
    // The remote to publish to
    remote: Remote<'a>,
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
        let e = GitError::All;
        tracing::error!(
            message = %e,
        );
        return Err(e);
    }

    // let client_req = context.remote.connect(Direction::Push);
    // let mut _client = client_req?;
    // tracing::info!(message = %_client.transport_mut().connection_persists_across_multiple_requests());

    Ok(())
}

impl<'a> PublishGitContext<'a> {
    async fn set(repo: &'a Repository, args: GitArgs) -> Result<Self, GitError> {
        let GitArgs { remote, spec } = args;
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

        Ok(Self {
            repo,
            tree,
            commit,
            remote,
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
                    path
                } else {
                    path.with_extension("atom")
                };
                self.repo.work_dir().map_or_else(
                    || self.publish_atom(&atom_path),
                    |rel_repo| self.publish_workdir_atom(rel_repo, &atom_path),
                )
            })
            .collect()
    }
}
