use toml_edit::de;

use super::*;

#[test]
fn deserialize() -> anyhow::Result<()> {
    let manifest = r#"
        [atom]
        id = "foo"
        version = "0.1.0"

        [deps.atoms.bar]
        version = "^1"
        path = "../atoms/bar"

        [deps.atoms.baz]
        version = "^0.3"
        url = "https://github.com/bar/baz.git"

        [deps.srcs.bin]
        path = "../srcs/bin"

        [deps.pins.pkgs]
        url = "https://github.com/nixos/nixpkgs.git"
        ref = "nixpkgs-unstable"
    "#;

    let atom = de::from_str::<Manifest>(manifest)?;
    toml_edit::ser::to_string_pretty(&atom)?;
    Ok(())
}
