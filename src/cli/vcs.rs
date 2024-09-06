use std::path::{Path, PathBuf};
use thiserror::Error;

#[cfg(feature = "git")]
use gix::{
    discover::{self, repository::Path as RepoPath},
    sec::Trust,
};

#[derive(Error, Debug, PartialEq, Eq)]
pub enum VcsError {
    #[error("No supported repository found in this directory or its parents")]
    None,
}

#[non_exhaustive]
pub enum Vcs {
    #[cfg(feature = "git")]
    Git(PathBuf),
}

pub fn detect() -> Result<Vcs, VcsError> {
    #[cfg(feature = "git")]
    {
        if let Ok((r, t)) = discover::upwards(Path::new(".")) {
            let path = match r {
                RepoPath::LinkedWorkTree {
                    work_dir,
                    git_dir: _,
                } => {
                    tracing::debug!(
                        message = "Detected linked Git worktree",
                        path = format!("{}", work_dir.display()),
                    );
                    work_dir
                }
                RepoPath::WorkTree(p) => {
                    tracing::debug!(
                        message = "Detected Git repository",
                        path = format!("{}", p.display()),
                    );
                    p
                }
                RepoPath::Repository(p) => {
                    tracing::warn!(
                        message = "Detected bare Git repository",
                        path = format!("{}", p.display()),
                    );
                    p
                }
            };

            match t {
                Trust::Reduced => tracing::warn!(
                    message = "Ignoring untrusted Git repository",
                    path = format!("{}", path.display()),
                ),
                Trust::Full => return Ok(Vcs::Git(path)),
            }
        }
    }

    Err(VcsError::None)
}
