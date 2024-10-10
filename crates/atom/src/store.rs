pub mod git;
use std::path::{Path, PathBuf};

use bstr::BStr;

pub trait Init<R> {
    type Error;
    fn sync(&self) -> Result<R, Self::Error>;
    fn ekala_init(&self) -> Result<(), Self::Error>;
    fn is_ekala_store(&self) -> bool;
}

pub trait NormalizeStorePath {
    type Error;
    /// Normalizes a given path to be relative to the store root.
    ///
    /// This function takes a path (relative or absolute) and attempts to normalize it
    /// relative to the store root, based on the current working directory within
    /// the store within system.
    ///
    /// # Behavior:
    /// - For relative paths (e.g., "foo/bar" or "../foo"):
    ///   - Interpreted as relative to the current working directory within the repository.
    ///   - Computed relative to the repository root.
    ///
    /// - For absolute paths (e.g., "/foo/bar"):
    ///   - Treated as if the repository root is the filesystem root.
    ///   - The leading slash is ignored, and the path is considered relative to the repo root.
    fn normalize<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf, Self::Error>;
}

trait QueryStore<Id> {
    type Error;
    fn get_refs<Spec>(
        &self,
        targets: impl IntoIterator<Item = Spec>,
    ) -> Result<impl IntoIterator<Item = Id>, Self::Error>
    where
        Spec: AsRef<BStr>;
    fn get_ref<Spec>(&self, target: Spec) -> Result<Id, Self::Error>
    where
        Spec: AsRef<BStr>;
}
