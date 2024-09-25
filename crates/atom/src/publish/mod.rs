pub mod error;
pub mod git;

use crate::{id::Id, AtomId};

use git::GitContent;
use std::path::PathBuf;

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
    /// This function processes a collection of paths, each representing an atom to be published. The publishing
    /// process includes path normalization, existence checks, and actual publishing attempts.
    ///
    /// # Path Normalization
    /// - First attempts to interpret each path as relative to the caller's current location inside the repository.
    /// - If normalization fails (e.g., in a bare repository), falls back to treating the path as already relative to the repo root.
    /// - The normalized path is used to search the Git history, not the file system.
    ///
    /// # Publishing Process
    /// For each path:
    /// 1. Normalizes the path (as described above).
    /// 2. Checks if the atom already exists in the repository.
    ///    - If it exists, the atom is skipped, and a log message is generated.
    /// 3. Attempts to publish the atom.
    ///    - If successful, the atom is added to the repository.
    ///    - If any error occurs during publishing, the atom is skipped, and an error is logged.
    ///
    /// # Error Handling
    /// - The function aims to process all provided paths, even if some fail.
    /// - Errors and skipped atoms are reported via logs but do not halt the overall process.
    /// - The function continues to the next atom after logging any errors or skip conditions.
    ///
    /// # Return Value
    /// Returns a vector of unit types (`Vec<()>`), primarily to indicate the number of paths processed.
    /// The caller should check the logs for detailed information on the success or failure of each atom.
    ///
    /// # Parameters
    /// - `paths`: A collection `C` of paths, each representing an atom to be published.
    ///
    /// # Note
    /// This function prioritizes completing the publishing attempt for all provided paths over
    /// stopping at the first error. Check the logs for a comprehensive report of the publishing process.
    fn publish<C>(&self, paths: C) -> Vec<Result<PublishOutcome<T>, Self::Error>>
    where
        C: IntoIterator<Item = PathBuf>;
}

impl<R> Record<R> {
    pub fn id(&self) -> &AtomId<R> {
        &self.id
    }
    pub fn content(&self) -> &Content {
        &self.content
    }
}
