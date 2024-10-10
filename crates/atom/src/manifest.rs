//! # Atom Manifest
//!
//! Provides the core types for working with an Atom's manifest format.
use crate::Atom;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;
use toml_edit::de;
use toml_edit::DocumentMut;

/// Errors which occur during manifest (de)serialization.
#[derive(Error, Debug)]
pub enum AtomError {
    /// The manifest is missing the required \[atom] key.
    #[error("Manifest is missing the `[atom]` key")]
    Missing,
    /// One of the fields in the required \[atom] key is missing or invalid.
    #[error(transparent)]
    InvalidAtom(#[from] de::Error),
    /// The manifest is not valid TOML.
    #[error(transparent)]
    InvalidToml(#[from] toml_edit::TomlError),
}

type AtomResult<T> = Result<T, AtomError>;

/// The type representing the required fields of an Atom's manifest.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Manifest {
    /// The required \[atom] key of the TOML manifest.
    pub atom: Atom,
}

impl Manifest {
    /// Build an Atom struct from the \[atom] key of a TOML manifest, ignoring other fields or keys].
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
