use gix::object;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PublishError {
    #[error(transparent)]
    Git(#[from] GitError),
}

#[derive(Error, Debug)]
pub enum GitError {
    #[error(transparent)]
    RemotNotFound(#[from] Box<gix::remote::find::existing::Error>),
    #[error(transparent)]
    RevParseFailed(#[from] Box<gix::revision::spec::parse::single::Error>),
    #[error(transparent)]
    NoCommit(#[from] object::find::existing::with_conversion::Error),
    #[error(transparent)]
    NoTree(#[from] object::commit::Error),
    #[error(transparent)]
    NoObject(#[from] object::find::existing::Error),
    #[error(transparent)]
    WriteFailed(#[from] object::write::Error),
    #[error(transparent)]
    RefUpdateFailed(#[from] gix::reference::edit::Error),
    #[error(transparent)]
    CalculatingRootFailed(#[from] gix::revision::walk::Error),
    #[error(transparent)]
    RootConversionFailed(#[from] gix::traverse::commit::simple::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    JoinFailed(#[from] tokio::task::JoinError),
    #[error("Remote is not initialized")]
    NotInitialized,
    #[error("Ignoring invalid atom manifest")]
    Invalid(#[source] crate::manifest::AtomError, Box<PathBuf>),
    #[error("The given path does not point to an atom")]
    NotAFile(PathBuf),
    #[error("Failed to sync some atoms to the remote")]
    SomePushFailed,
    #[error("Failed to published some of the specified atoms")]
    Failed,
    #[error(transparent)]
    StoreError(#[from] crate::store::git::Error),
    #[error("Failed to find any atoms under the current directory")]
    NotFound,
    #[error("Duplicate atoms detected in the given revision, refusing to publish")]
    Duplicates,
}

impl GitError {
    pub fn warn(&self) {
        match self {
            GitError::Invalid(e, path) => {
                tracing::warn!(message = %self, path = %path.display(), message = format!("\n{}", e))
            }
            GitError::NotAFile(path) => {
                tracing::warn!(message = %self, path = %path.display())
            }
            GitError::Failed => (),
            _ => tracing::warn!(message = %self),
        }
    }
}
