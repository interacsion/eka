#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use thiserror::Error;
use unic_ucd_category::GeneralCategory;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Id(String);

#[derive(Error, Debug, PartialEq, Eq)]
pub enum IdError {
    #[error("An atom id cannot be empty")]
    Empty,
    #[error("An atom id cannot start with a number, apostrophe, dash or underscore")]
    InvalidStart,
    #[error("The atom id contains invalid characters: '{0}'")]
    InvalidCharacters(String),
}

impl Id {
    fn validate_start(c: char) -> Result<(), IdError> {
        if Id::is_invalid_start(c) {
            return Err(IdError::InvalidStart);
        }
        Ok(())
    }

    pub(super) fn validate(s: &str) -> Result<(), IdError> {
        match s.chars().next().map(Id::validate_start) {
            Some(Ok(_)) => (),
            Some(Err(e)) => return Err(e),
            None => return Err(IdError::Empty),
        }

        let invalid_chars: String = s.chars().filter(|&c| !Id::is_valid_char(c)).collect();

        if !invalid_chars.is_empty() {
            return Err(IdError::InvalidCharacters(invalid_chars));
        }

        Ok(())
    }
    pub(super) fn is_invalid_start(c: char) -> bool {
        matches!(
            GeneralCategory::of(c),
            GeneralCategory::DecimalNumber | GeneralCategory::LetterNumber
        ) || c == '_'
            || c == '-'
            || !Id::is_valid_char(c)
    }
    pub(super) fn is_valid_char(c: char) -> bool {
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
    }
}

impl Deref for Id {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl FromStr for Id {
    type Err = IdError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Id::validate(s)?;
        Ok(Id(s.to_string()))
    }
}

impl TryFrom<String> for Id {
    type Error = IdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Id::validate(&s)?;
        Ok(Id(s))
    }
}

impl TryFrom<&str> for Id {
    type Error = IdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Id::from_str(s)
    }
}
