use atom::store::git;
use thiserror::Error;

#[cfg(feature = "git")]
use gix::ThreadSafeRepository;

#[non_exhaustive]
#[derive(Clone, Debug)]
pub(super) enum Detected {
    #[cfg(feature = "git")]
    Git(&'static ThreadSafeRepository),
}

#[tracing::instrument(err)]
pub(super) async fn detect() -> Result<Detected, StoreError> {
    #[cfg(feature = "git")]
    {
        if let Ok(Some(repo)) = git::repo() {
            use std::fs;
            let git_dir = fs::canonicalize(repo.path())
                .ok()
                .map(|p| p.display().to_string());
            let work_dir = repo
                .work_dir()
                .and_then(|dir| fs::canonicalize(dir).ok())
                .map(|p| p.display().to_string());

            tracing::debug!(message = "Detected Git repository", git_dir, work_dir);
            return Ok(Detected::Git(repo));
        }
    }

    #[cfg_attr(feature = "git", allow(unreachable_code))]
    Err(StoreError::FailedDetection)
}

#[derive(Error, Debug)]
pub(crate) enum StoreError {
    #[error("No supported repository found in this directory or its parents")]
    FailedDetection,
    #[cfg(feature = "git")]
    #[error(transparent)]
    Discover(#[from] Box<gix::discover::Error>),
}
