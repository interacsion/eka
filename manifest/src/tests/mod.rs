use super::core::Manifest;

#[test]
fn serialize() -> anyhow::Result<()> {
    let results: Vec<Manifest> = vec![
        r#"
            trait = "package"
            [atom]
            id = "foo"
            version = "0.1.0"
        "#
        .parse()?,
        r#"
            trait = "deployment"
            [atom]
            id = "bar"
            version = "0.2.0"
        "#
        .parse()?,
        r#"
            trait = "config"
            [atom]
            id = "some"
            version = "1.2.0"
            description = "Some config"
        "#
        .parse()?,
    ];

    insta::assert_yaml_snapshot!(results);
    Ok(())
}
