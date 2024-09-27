use super::id::Id;

use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Represents the deserialized form of an Atom, directly constructed from the TOML manifest.
///
/// This struct contains the basic metadata of an Atom but lacks the context-specific
/// `AtomId`, which must be constructed separately.
pub struct Atom {
    /// The verified Unicode identifier for the atom.
    pub id: Id,

    /// The version of the atom.
    pub version: Version,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// An optional description of the atom.
    pub description: Option<String>,
}
