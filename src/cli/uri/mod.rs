#[cfg(test)]
mod tests;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    character::complete::digit1,
    combinator::{map, not, opt, peek, recognize, verify},
    sequence::tuple,
    IResult,
};

#[cfg(test)]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(test, derive(Serialize, Deserialize))]
pub struct UriRef<'a> {
    scheme: Option<&'a str>,
    alias: Option<&'a str>,
    frag: Option<&'a str>,
    atom: Option<&'a str>,
    version: Option<&'a str>,
}

impl<'a> From<&'a str> for UriRef<'a> {
    /// Parse a string representation of an atom URI into a struct of its parts.
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
                            recognize(tuple((take_until("@"), take_until("."), take_until("/")))),
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
