use super::Name;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use toml_edit::de;
use toml_edit::DocumentMut;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Manifest {
    r#trait: Name,
    pub atom: Atom,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Atom {
    id: Name,
    version: Version,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl Manifest {
    pub fn is(content: &str) -> bool {
        let doc = match content.parse::<DocumentMut>() {
            Ok(doc) => doc,
            Err(_) => return false,
        };

        if let Some(v) = doc.get("atom").and_then(|v| v.as_str()) {
            if de::from_str::<Atom>(v).is_ok() {
                return true;
            }
        }

        false
    }
}

impl FromStr for Manifest {
    type Err = de::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        de::from_str(s)
    }
}
