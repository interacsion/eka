use super::core::Manifest;
use super::Name;
use semver::VersionReq;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Depend {
    #[serde(flatten)]
    core: Manifest,
    deps: HashMap<Name, Dependency>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Dependency {
    version: VersionReq,
    repo: Url,
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml_edit::de::from_str;

    #[test]
    fn de_depend() -> anyhow::Result<()> {
        let atom_str = r#"
            trait = "package"
            [atom]
            name = "foo"
            version = "0.1.0"

            [deps.foo]
            version = "^1"
            repo = "https://example.com/foo/bar.git"
        "#;

        from_str::<Depend>(atom_str)?;

        Ok(())
    }
}
