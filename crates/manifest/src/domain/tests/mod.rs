use super::*;
use toml_edit::de::from_str;

#[test]
fn depend() -> anyhow::Result<()> {
    let atom_str = r#"
            trait = "package"
            [atom]
            id = "foo"
            version = "0.1.0"

            [deps.foo]
            version = "^1"
            repo = "https://example.com/foo/bar.git"
        "#;

    insta::assert_yaml_snapshot!([from_str::<Depend>(atom_str)?]);

    Ok(())
}
