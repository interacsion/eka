use super::Name;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;
use toml_edit::de;
use toml_edit::DocumentMut;

#[derive(Error, Debug)]
pub enum AtomError {
    #[error("Manifest is missing the `[atom]` key")]
    Missing,
    #[error(transparent)]
    InvalidAtom(#[from] de::Error),
    #[error(transparent)]
    InvalidToml(#[from] toml_edit::TomlError),
}

type AtomResult<T> = Result<T, AtomError>;

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
    /// Get the Atom object from a toml manifest
    pub fn get_atom(content: &str) -> AtomResult<Atom> {
        let doc = content.parse::<DocumentMut>()?;

        if let Some(v) = doc.get("atom").map(|v| v.to_string()) {
            let atom = de::from_str::<Atom>(&v)?;
            Ok(atom)
        } else {
            Err(AtomError::Missing)
        }
    }
}

impl FromStr for Manifest {
    type Err = de::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        de::from_str(s)
    }
}
