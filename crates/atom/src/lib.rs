//! # Atom Crate
//!
//! The `atom` crate provides the functionality for working with the Atom Format,
//! a key component of the Ekala Project. This format enables the reproducible
//! packaging of select sources from a larger history.
//!
//! It is purposely designed to be cheap to transfer over the network, and trivial
//! to verify directly from source.
//!
//! ## Git Example
//! The inaugural implementation uses Git refs pointing to orphaned histories of
//! an individual directory from a commit, as well as a manifest describing its
//! contents. Here is an example of a single published Atom in Git.
//!
//! ```console
//! ❯ git ls-remote
//! From https://github.com/ekala-project/eka
//! ceebaca6d44c4cda555db3fbf687c0604c4818eb        refs/atoms/ひらがな/0.1.0
//! a87bff5ae43894a158dadf40938c775cb5b62d4b        refs/atoms/ひらがな/_specs/0.1.0
//! 9f17c8c816bd1de6f8aa9c037d1b529212ab2a02        refs/atoms/ひらがな/_srcs/0.1.0
//! ```
//!
//! Here the 0.1.0 ref points to the Atom's contents in full. The `_spec` refs points
//! to a git tree object containing only the manifest and its lock file, which will be
//! important for efficient resolution (not yet implemented). The refs under `_srcs`
//! point to the original commit from which the Atom's content references, ensuring
//! it remains live. Ensuring we can trivially verify an Atom's content at any time.
#![deny(missing_docs)]

mod core;
mod id;
mod manifest;

pub mod publish;
pub mod store;
pub mod uri;
pub use core::Atom;
pub use id::AtomId;
pub use id::CalculateRoot;
pub use manifest::Manifest;

use std::sync::LazyLock;
const TOML: &str = "toml";
const BASE32: base32::Alphabet = base32::Alphabet::Rfc4648HexLower { padding: false };
static ATOM_EXT: LazyLock<String> = LazyLock::new(|| format!("@.{}", crate::TOML));
