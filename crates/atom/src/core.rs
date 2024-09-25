use super::id::Id;

use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Atom {
    pub id: Id,
    pub version: Version,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
