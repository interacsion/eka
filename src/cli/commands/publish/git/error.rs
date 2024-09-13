use gix::object;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum GitError {
    #[error(transparent)]
    RemotNotFound(#[from] gix::remote::find::existing::Error),
    #[error(transparent)]
    RevParseFailed(#[from] gix::revision::spec::parse::single::Error),
    #[error(transparent)]
    NoCommit(#[from] object::find::existing::with_conversion::Error),
    #[error(transparent)]
    NoTree(#[from] object::commit::Error),
    #[error(transparent)]
    PushFailed(#[from] std::io::Error),
    #[error(transparent)]
    JoinFailed(#[from] tokio::task::JoinError),
    #[error("Failed to sync some atoms to the remote")]
    SomePushFailed,
    #[error("Failed to published any of the specified atoms")]
    AllFailed,
}
