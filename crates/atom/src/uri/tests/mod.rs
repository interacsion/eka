use super::*;

const ALIASES: &[&str] = &[
    "gl:repo::atom@^2.0",
    "gl::atom@^2.1",
    "gl:path.with/dot::my-atom@^2",
    "git@github.com:owner/repo::this-atom@^1",
    "git@gh:owner/repo::this-atom@^1",
    "https://example.com:8080/owner/repo::foo@^1",
    "https://foo.com/owner/repo::bar@^1",
    "https://gh:owner/repo::λ@^1",
    "gh:owner/repo::λ@^1",
    "git@gh:owner/repo::λ@^1",
    "pkgs::zlib@^1",
    "https://user:password@example.com/my/repo::id@^0.2",
    "user:password@example.com/my/repo::id@^0.2",
    "user@example.com/my/repo::id@^0.2",
    "gh:owner/repo::yep@^1",
    "bb::yep@^1",
    "foo/bar/baz::my-atom",
    "/foo/bar/baz::my-atom",
    // not an alias, but an ssh url without a username
    "my.ssh.com:my/repo::hello",
    "foo@^0.8",
    "::foo",
    "::foo",
];

#[test]
fn ref_snapshot() {
    let results: Vec<Ref> = ALIASES.iter().map(|x| (*x).into()).collect();
    insta::assert_yaml_snapshot!(results);
}

#[test]
fn uri_snapshot() -> Result<(), UriError> {
    let results: Result<Vec<Uri>, UriError> = ALIASES.iter().map(|v| v.parse::<Uri>()).collect();
    insta::assert_debug_snapshot!(results?);
    Ok(())
}
