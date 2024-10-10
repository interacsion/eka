use super::id::Id;

use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Represents the deserialized form of an Atom, directly constructed from the TOML manifest.
///
/// This struct contains the basic metadata of an Atom but lacks the context-specific
/// [`crate::AtomId`], which must be constructed separately.
pub struct Atom {
    /// The verified, human-readable Unicode identifier for the Atom.
    pub id: Id,

    /// The version of the Atom.
    pub version: Version,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// An optional description of the Atom.
    pub description: Option<String>,
}
