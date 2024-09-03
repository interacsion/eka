use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    character::complete::digit1,
    combinator::{map, not, opt, peek, recognize, verify},
    sequence::tuple,
    IResult,
};
use serde::{Deserialize, Serialize};

// TODO: not complete
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct UriRef<'a> {
    scheme: Option<&'a str>,
    alias: Option<&'a str>,
    frag: Option<&'a str>,
    atom: Option<&'a str>,
    version: Option<&'a str>,
}

impl<'a> From<&'a str> for UriRef<'a> {
    fn from(input: &'a str) -> Self {
        let empty = |(rest, opt): (&'a _, Option<&'a str>)| {
            (
                rest,
                opt.and_then(|x| if x.is_empty() { None } else { Some(x) }),
            )
        };
        let scheme = |input: &'a str| -> IResult<&'a str, Option<&'a str>> {
            opt(map(
                tuple((take_until("://"), tag("://"))),
                |(scheme, _)| scheme,
            ))(input)
            .map(empty)
        };

        let alias = |input: &'a str| {
            opt(map(
                verify(
                    tuple((
                        take_until(":"),
                        tag(":"),
                        peek(not(alt((
                            // not a port
                            digit1,
                            // not a user:pass@example.com
                            recognize(tuple((take_until("@"), take_until(".")))),
                        )))),
                    )),
                    |(before_colon, _, _)| {
                        #[allow(clippy::explicit_auto_deref)]
                        // not an SSH-style url
                        <str>::find(*before_colon, |c| c == '@' || c == '.').is_none()
                    },
                ),
                |(alias, _, _)| alias,
            ))(input)
        };

        let frag = |input: &'a str| {
            opt(map(tuple((take_until("//"), tag("//"))), |(frag, _)| frag))(input).map(empty)
        };

        let atom = |input: &'a str| {
            opt(map(
                alt((
                    tuple((take_until("@"), tag("@"))),
                    // consume the rest of the input if there is no version tag (`@`)
                    tuple((take_while(|_| true), tag(""))),
                )),
                |(alias, _)| alias,
            ))(input)
            .map(empty)
        };

        let version = |input: &'a str| opt(take_while(|_| true))(input).map(empty);

        let (_rem, (scheme, alias, frag, atom, version)) =
            tuple((scheme, alias, frag, atom, version))(input).unwrap_or_default();

        UriRef {
            scheme,
            alias,
            frag,
            atom,
            version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uri_snapshot() {
        let results: Vec<UriRef> = vec![
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
}
