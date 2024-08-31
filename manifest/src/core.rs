use super::Name;
use semver::Version;
use serde::{Deserialize, Serialize};
use toml_edit::de::from_str;
use toml_edit::DocumentMut;
use url::Url;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Manifest {
    r#trait: Name,
    pub atom: Atom,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Atom {
    name: Name,
    version: Version,
    description: Option<String>,
    canon: Option<Url>,
}

impl Manifest {
    pub fn is(content: &str) -> bool {
        let doc = match content.parse::<DocumentMut>() {
            Ok(doc) => doc,
            Err(_) => return false,
        };

        if let Some(v) = doc.get("atom").and_then(|v| v.as_str()) {
            if from_str::<Atom>(v).is_ok() {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml_edit::de::from_str;

    #[test]
    fn serde() -> anyhow::Result<()> {
        let atom_str = r#"
            trait = "package"
            [atom]
            name = "foo"
            version = "0.1.0"
        "#;

        let atom = Manifest {
            r#trait: "package".parse()?,
            atom: Atom {
                name: "foo".parse()?,
                version: "0.1.0".parse()?,
                description: None,
                canon: None,
            },
        };

        let atom2: Manifest = from_str(atom_str)?;

        assert_eq!(atom2, atom);
        Ok(())
    }
}
