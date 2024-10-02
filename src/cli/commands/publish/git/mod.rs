use super::PublishArgs;

use atom::publish::{
    error::GitError,
    git::{GitOutcome, GitResult},
};
use clap::Parser;
use std::collections::HashSet;
use std::path::PathBuf;

use gix::ThreadSafeRepository;

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

pub(super) async fn run(
    repo: ThreadSafeRepository,
    args: PublishArgs,
) -> GitResult<(Vec<GitResult<GitOutcome>>, Vec<GitError>)> {
    use atom::{
        publish::{git::GitPublisher, Builder, Publish},
        store::NormalizeStorePath,
    };
    use std::path::Path;
    let repo = repo.to_thread_local();

    let GitArgs { remote, spec } = args.store.git;

    let (atoms, publisher) = GitPublisher::new(&repo, &remote, &spec).build()?;

    let mut errors = Vec::with_capacity(args.path.len());
    let results = if args.recursive {
        let paths: HashSet<_> = if !repo.is_bare() {
            let cwd = repo.normalize(repo.current_dir())?;
            atoms
                .into_values()
                .filter_map(|path| path.strip_prefix(&cwd).map(Path::to_path_buf).ok())
                .collect()
        } else {
            atoms.into_values().collect()
        };

        if paths.is_empty() {
            return Err(GitError::NotFound);
        }
        publisher.publish(paths)
    } else {
        // filter redundant paths
        let paths: HashSet<PathBuf> = args.path.into_iter().collect();
        publisher.publish(paths)
    };

    publisher.await_pushes(&mut errors).await;

    Ok((results, errors))
}
