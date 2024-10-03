use crate::id::CalculateRoot;

use gix::{
    discover::upwards::Options,
    sec::{trust::Mapping, Trust},
    Commit, ObjectId, ThreadSafeRepository,
};
use std::sync::OnceLock;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Repository does not have a working directory")]
    NoWorkDir,
    #[error("Failed to calculate the repositories root commit")]
    RootNotFound,
    #[error(transparent)]
    WalkFailure(#[from] gix::revision::walk::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    NormalizationFailed(#[from] std::path::StripPrefixError),
}

/// Provide a lazyily instantiated static reference to the git repository.
static REPO: OnceLock<Option<ThreadSafeRepository>> = OnceLock::new();

pub struct Root(ObjectId);

pub fn repo() -> Result<Option<&'static ThreadSafeRepository>, Box<gix::discover::Error>> {
    let mut error = None;
    let repo = REPO.get_or_init(|| match get_repo() {
        Ok(repo) => Some(repo),
        Err(e) => {
            error = Some(e);
            None
        }
    });
    if let Some(e) = error {
        Err(e)
    } else {
        Ok(repo.as_ref())
    }
}

fn get_repo() -> Result<ThreadSafeRepository, Box<gix::discover::Error>> {
    let opts = Options {
        required_trust: Trust::Full,
        ..Default::default()
    };
    ThreadSafeRepository::discover_opts(".", opts, Mapping::default()).map_err(Box::new)
}

pub fn default_remote() -> String {
    use gix::remote::Direction;
    repo()
        .ok()
        .flatten()
        .and_then(|repo| {
            repo.to_thread_local()
                .remote_default_name(Direction::Push)
                .map(|s| s.to_string())
        })
        .unwrap_or("origin".into())
}

use std::ops::Deref;
impl Deref for Root {
    type Target = ObjectId;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> CalculateRoot<Root> for Commit<'a> {
    type Error = Error;
    fn calculate_root(&self) -> Result<Root, Self::Error> {
        use gix::traverse::commit::simple::{CommitTimeOrder, Sorting};
        // FIXME: we rely on a custom crate patch to search the commit graph
        // with a bias for older commits. The default gix behavior is the opposite
        // starting with bias for newer commits.
        //
        // it is based on the more general concept of an OldestFirst traversal
        // introduce by @nrdxp upstream: https://github.com/Byron/gitoxide/pull/1610
        //
        // However, that work tracks main and the goal of this patch is to remain
        // as minimal as possible on top of a release tag, for easier maintenance
        // assuming it may take a while to merge upstream.
        let mut walk = self
            .ancestors()
            .use_commit_graph(true)
            .sorting(Sorting::ByCommitTime(CommitTimeOrder::OldestFirst))
            .all()?;

        while let Some(Ok(info)) = walk.next() {
            if info.parent_ids.is_empty() {
                return Ok(Root(info.id));
            }
        }

        Err(Error::RootNotFound)
    }
}

use super::NormalizeStorePath;
use gix::Repository;
use std::path::{Path, PathBuf};

impl NormalizeStorePath for Repository {
    type Error = Error;
    fn normalize<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf, Error> {
        use path_clean::PathClean;
        use std::fs;
        let path = path.as_ref();

        let rel_repo_root = self.work_dir().ok_or(Error::NoWorkDir)?;
        let repo_root = fs::canonicalize(rel_repo_root)?;
        let current = self.current_dir();
        let rel = current.join(path).clean();

        rel.strip_prefix(&repo_root)
            .map_or_else(
                |e| {
                    // handle absolute paths as if they were relative to the repo root
                    if !path.is_absolute() {
                        return Err(e);
                    }
                    let cleaned = path.clean();
                    // Preserve the platform-specific root
                    let p = cleaned.strip_prefix(Path::new("/"))?;
                    repo_root
                        .join(p)
                        .clean()
                        .strip_prefix(&repo_root)
                        .map(Path::to_path_buf)
                },
                |p| Ok(p.to_path_buf()),
            )
            .map_err(|e| {
                tracing::warn!(
                    message = "Ignoring path outside repo root",
                    path = %path.display(),
                );
                Error::NormalizationFailed(e)
            })
    }
}

impl AsRef<[u8]> for Root {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
