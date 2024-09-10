mod r#impl;

use super::PublishArgs;
use crate::cli::logging::LogValue;

use clap::Parser;
use std::path::PathBuf;
use thiserror::Error;

use gix::{
    discover, remote::find::existing, Commit, Reference, Remote, Repository, ThreadSafeRepository,
    Tree,
};

#[derive(Error, Debug)]
enum GitError {
    #[error(transparent)]
    Discover(#[from] discover::Error),
    #[error(transparent)]
    RemotNotFound(#[from] existing::Error),
}

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

struct PublishGitContext<'a> {
    repo: &'a Repository,
    tree: Tree<'a>,
    commit: Commit<'a>,
    remote: Remote<'a>,
}

pub(super) async fn run(repo: ThreadSafeRepository, args: PublishArgs) -> anyhow::Result<()> {
    let repo = repo.to_thread_local();

    let context = PublishGitContext::new(&repo, args.vcs.git).await?;

    let atoms: Vec<Reference> = if args.recursive {
        todo!();
    } else {
        context.publish(args.path)
    };
    tracing::info!(message = ?atoms);

    Ok(())
}

impl<'a> PublishGitContext<'a> {
    async fn new(repo: &'a Repository, args: GitArgs) -> anyhow::Result<Self> {
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

        let tree = commit.tree()?;

        Ok(Self {
            repo,
            tree,
            commit,
            remote,
        })
    }

    fn publish<C>(&self, paths: C) -> Vec<Reference>
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
