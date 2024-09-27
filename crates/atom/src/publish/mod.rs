pub mod error;
pub mod git;

use crate::{id::Id, AtomId};

use git::GitContent;
use std::path::{Path, PathBuf};

pub struct Record<R> {
    id: AtomId<R>,
    content: Content,
}

/// A Result is used over an Option here mainly so we can report which
/// atom was skipped, but it does not represent a true failure condition
type MaybeSkipped<T> = Result<T, Id>;

/// A Record that signifies whether an atom was published or safetly skipped.
type PublishOutcome<R> = MaybeSkipped<Record<R>>;

pub enum Content {
    Git(GitContent),
}

pub trait Publish<T> {
    type Error;

    /// Publishes atoms.
    ///
    /// This function processes a collection of paths, each representing an atom to be published.
    ///
    /// # Error Handling
    /// - The function aims to process all provided paths, even if some fail.
    /// - Errors and skipped atoms are collected as results but do not halt the overall process.
    /// - The function continues until all the atoms have been processed.
    ///
    /// # Return Value
    /// Returns a vector of results types (`Vec<Result<PublishOutcome<T>, Self::Error>>`), where the
    /// outter result represents whether an atom has failed, and the inner result determines whether an
    /// atom was safely skipped, e.g. because it already exists..
    fn publish<C>(&self, paths: C) -> Vec<Result<PublishOutcome<T>, Self::Error>>
    where
        C: IntoIterator<Item = PathBuf>;

    fn publish_atom<P: AsRef<Path>>(&self, path: P) -> Result<PublishOutcome<T>, Self::Error>;
}

impl<R> Record<R> {
    pub fn id(&self) -> &AtomId<R> {
        &self.id
    }
    pub fn content(&self) -> &Content {
        &self.content
    }
}
