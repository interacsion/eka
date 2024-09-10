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
    #[error("Failed to published any specified atoms")]
    All,
}
