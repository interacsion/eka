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
pub(super) enum Vcs {
    #[cfg(feature = "git")]
    Git(ThreadSafeRepository),
}

#[tracing::instrument(err)]
pub(super) fn detect() -> Result<Vcs, VcsError> {
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
        return Ok(Vcs::Git(repo));
    }

    // TODO: not needed until we have another supported VCS
    // Err(VcsError::FailedDetection)
}

#[derive(Error, Debug)]
pub(crate) enum VcsError {
    // #[error("No supported repository found in this directory or its parents")]
    // FailedDetection,
    #[error(r#""{0}""#)]
    Discover(#[from] gix::discover::Error),
}
