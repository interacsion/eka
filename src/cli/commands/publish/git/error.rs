use gix::object;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum GitError {
    #[error(transparent)]
    RemotNotFound(#[from] gix::remote::find::existing::Error),
    #[error(transparent)]
    Parse(#[from] gix::revision::spec::parse::single::Error),
    #[error(transparent)]
    Commit(#[from] object::find::existing::with_conversion::Error),
    #[error(transparent)]
    Tree(#[from] object::commit::Error),
    #[error(transparent)]
    Push(#[from] std::io::Error),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error("Failed to sync some atoms to the remote")]
    Pushes,
    #[error("Failed to published any of the specified atoms")]
    All,
}
