#[cfg(test)]
mod tests;

pub mod core;
mod domain;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use thiserror::Error;
use unic_ucd_category::GeneralCategory;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord)]
#[serde(try_from = "String")]
pub struct Name(String);

#[derive(Error, Debug, PartialEq, Eq)]
pub enum NameError {
    #[error("The `name` field cannot be empty")]
    Empty,
    #[error("The `name` field cannot start with a number, apostrophe, dash or underscore")]
    InvalidStart,
    #[error("The `name` field contains invalid characters: '{0}'")]
    InvalidCharacters(String),
}

impl Name {
    fn validate(s: &str) -> Result<(), NameError> {
        if s.is_empty() {
            return Err(NameError::Empty);
        }

        if let Some(c) = s.chars().next() {
            if matches!(
                GeneralCategory::of(c),
                GeneralCategory::DecimalNumber | GeneralCategory::LetterNumber
            ) || c == '_'
                || c == '-'
                || c == '\''
            {
                return Err(NameError::InvalidStart);
            }
        }

        let invalid_chars: String = s.chars().filter(|&c| !Name::is_valid_char(c)).collect();

        if !invalid_chars.is_empty() {
            return Err(NameError::InvalidCharacters(invalid_chars));
        }

        Ok(())
    }
    fn is_valid_char(c: char) -> bool {
        matches!(
            GeneralCategory::of(c),
            GeneralCategory::LowercaseLetter
                | GeneralCategory::UppercaseLetter
                | GeneralCategory::TitlecaseLetter
                | GeneralCategory::ModifierLetter
                | GeneralCategory::OtherLetter
                | GeneralCategory::DecimalNumber
                | GeneralCategory::LetterNumber
        ) || c == '-'
            || c == '_'
            || c == '\''
    }
}

impl Deref for Name {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl FromStr for Name {
    type Err = NameError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Name::validate(s)?;
        Ok(Name(s.to_string()))
    }
}

impl TryFrom<String> for Name {
    type Error = NameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Name::from_str(&value)
    }
}
