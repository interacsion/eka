use serde::{Deserialize, Serialize};
use std::fmt;
use url::Url;

type Name = String;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Lockfile {
    pub version: u8,
    #[serde(flatten)]
    pub schema: LockSchema,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum LockSchema {
    V1(LockV1),
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct LockV1 {
    pub dep: Vec<Locked>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Locked {
    pub name: Name,
    pub repo: Url,
    pub sum: Sha1Hash,
    pub deps: Option<Vec<Name>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha1Hash([u8; 20]);

impl TryFrom<&str> for Sha1Hash {
    type Error = anyhow::Error;

    fn try_from(hex_str: &str) -> Result<Self, Self::Error> {
        if hex_str.len() != 40 {
            return Err(anyhow::anyhow!("SHA-1 hash must be 40 characters long"));
        }

        let mut bytes = [0u8; 20];
        hex::decode_to_slice(hex_str, &mut bytes as &mut [u8])?;

        Ok(Sha1Hash(bytes))
    }
}

impl TryFrom<String> for Sha1Hash {
    type Error = anyhow::Error;

    fn try_from(hex_str: String) -> Result<Self, Self::Error> {
        Self::try_from(hex_str.as_str())
    }
}

impl Serialize for Sha1Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Sha1Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = Self::try_from(s).map_err(serde::de::Error::custom)?;
        Ok(bytes)
    }
}

impl fmt::Display for Sha1Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_lock() -> anyhow::Result<()> {
        macro_rules! svec {
            ($($x:expr),*) => (vec![$($x.to_string()),*]);
        }

        let orig_string = r#"
version = 1

[[dep]]
name = "foo"
repo = "https://github.com/ekala-project/atom.git"
sum = "318a942f39b56f6e9af878564f883d43307ceb87"
deps = ["bar", "baz", "buz 0.2"]
"#
        .trim_start();

        let orig = Lockfile {
            version: 1,
            schema: LockSchema::V1(LockV1 {
                dep: vec![Locked {
                    name: "foo".to_owned(),
                    repo: Url::parse("https://github.com/ekala-project/atom.git")?,
                    sum: Sha1Hash::try_from("318a942f39b56f6e9af878564f883d43307ceb87")?,
                    deps: Some(svec!["bar", "baz", "buz 0.2"]),
                }],
            }),
        };
        let string = toml::to_string(&orig)?;

        let lock: Lockfile = toml::from_str(string.as_str())?;
        assert_eq!(orig_string, string);
        assert_eq!(orig, lock);
        Ok(())
    }
}
