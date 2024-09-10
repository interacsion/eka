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
    pub id: Name,
    pub version: Version,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Manifest {
    pub fn is(content: &str) -> anyhow::Result<Atom> {
        let doc = content.parse::<DocumentMut>()?;

        if let Some(v) = doc.get("atom").map(|v| v.to_string()) {
            let atom = de::from_str::<Atom>(&v)?;
            Ok(atom)
        } else {
            // TODO: make a proper error type
            Err(anyhow::format_err!("Manifest is missing the `[atom]` key"))
        }
    }
}

impl FromStr for Manifest {
    type Err = de::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        de::from_str(s)
    }
}
