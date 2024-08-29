use super::Name;
use semver::Version;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Manifest {
    r#trait: Name,
    #[serde(flatten)]
    category: Category,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Core {
    name: Name,
    version: Version,
    description: Option<String>,
    /// The canonical source repository this code is developed in.
    /// If ommitted, the source will not be exposed for public discovery
    /// and will therefore not be usable as a remote dependency.
    canon: Option<Url>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Category {
    /// The Atom domain represents the minimal schema abstraction in eka. While its exact
    /// function would be determined by its set trait, it is generally meant to encompass a
    /// single concern: a package, a system configuration, a service definition, a deployment.
    Atom(Core),
    Lattice(Core),
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
            category: Category::Atom(Core {
                name: "foo".parse()?,
                version: "0.1.0".parse()?,
                description: None,
                canon: None,
            }),
        };

        let atom2: Manifest = from_str(atom_str)?;

        assert_eq!(atom2, atom);
        Ok(())
    }
}
