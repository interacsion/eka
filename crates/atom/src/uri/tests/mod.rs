use super::*;
#[test]
fn uri_snapshot() {
    let results: Vec<Ref> = vec![
        "alias:repo//atom@^2.0".into(),
        "alias:atom@^2.1".into(),
        "alias:path.with/dot//my/atom@^2".into(),
        "git@github.com:owner/repo//path/to/atom@^1".into(),
        "https://example.com/owner/repo:8080//path/to/atom@^1".into(),
        "https://github.com/owner/repo//path/to/atom@^1".into(),
        "https://hub:owner/repo//path/to/atom@^1".into(),
        "https://user:password@example.com/my/repo//atom/path@^0.2".into(),
        "hub:owner/repo//path/to/atom@^1".into(),
        // not an alias, but an ssh url without a username
        "my.ssh.com:my/repo//path/to/atom".into(),
        "/path/to/atom@^0.8".into(),
        "///path/to/atom".into(),
        "//path/to/atom".into(),
    ];
    insta::assert_yaml_snapshot!(results);
}
