use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    character::complete::digit1,
    combinator::{map, not, opt, peek, verify},
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
                    tuple((take_until(":"), tag(":"), peek(not(alt((digit1,)))))),
                    |(before_colon, _, _)| {
                        #[allow(clippy::explicit_auto_deref)]
                        // Fail if it's an SSH-style URL
                        !<str>::contains(*before_colon, '@')
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
            "https://github.com/owner/repo//path/to/atom@^1".into(),
            "https://hub:owner/repo//path/to/atom@^1".into(),
            "hub:owner/repo//path/to/atom@^1".into(),
            "git@github.com:owner/repo//path/to/atom@^1".into(),
            "///path/to/atom".into(),
            "//path/to/atom".into(),
            "/path/to/atom@^0.8".into(),
        ];
        insta::assert_yaml_snapshot!(results);
    }
}
