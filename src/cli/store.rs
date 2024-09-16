use super::logging::LogValue;
use thiserror::Error;

#[cfg(feature = "git")]
use gix::{
    discover::upwards::Options,
    sec::{trust::Mapping, Trust},
    ThreadSafeRepository,
};

#[non_exhaustive]
#[derive(Clone, Debug)]
pub(super) enum Store {
    #[cfg(feature = "git")]
    Git(ThreadSafeRepository),
}

#[tracing::instrument(err)]
pub(super) fn detect() -> Result<Store, StoreError> {
    #[cfg(feature = "git")]
    {
        let opts = Options {
            required_trust: Trust::Full,
            ..Default::default()
        };
        let repo = ThreadSafeRepository::discover_opts(".", opts, Mapping::default())?;
        tracing::debug!(
            message = "Detected Git repository",
            git_dir = %repo.path().as_json(),
            work_dir = %repo.work_dir().as_json()
        );
        return Ok(Store::Git(repo));
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
    Discover(#[from] gix::discover::Error),
}
