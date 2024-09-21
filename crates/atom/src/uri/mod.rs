#[cfg(test)]
mod tests;

use std::ops::Deref;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::id::IdError;

use super::Id;

use gix::Url;
use semver::VersionReq;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug)]
struct Aliases(HashMap<&'static str, &'static str>);

#[derive(Debug)]
struct Parser<'a> {
    aliases: Aliases,
    refs: Ref<'a>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Uri {
    // The URL to the repository containing the atom
    url: Option<Url>,
    // The atom id to be located
    id: Id,
    // the requested atom version
    version: Option<VersionReq>,
}

impl std::fmt::Display for Uri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let url = self.url.as_ref().map(|u| u.to_string()).unwrap_or_default();
        let version = self
            .version
            .as_ref()
            .map(|v| format!("@{}", v))
            .unwrap_or_default();
        write!(f, "{}::{}{}", &url, self.id, &version)
    }
}

impl Uri {
    pub fn url(&self) -> Option<&Url> {
        self.url.as_ref()
    }
    pub fn id(&self) -> &Id {
        &self.id
    }
    pub fn version(&self) -> Option<&VersionReq> {
        self.version.as_ref()
    }
}

/// Represents the parsed components of an atom reference URI.
///
/// This struct is an intermediate representation resulting from parsing a URI string
/// in the format: `[scheme://][alias:][url-fragment::]atom-id[@version]`
///
/// It is typically created through the `From<&str>` implementation, not constructed directly.
///
/// # Components
///
/// * `scheme`: The URI scheme (e.g., "https", "ssh"). Optional.
/// * `user`: The username. Optional.
/// * `pass`: The password. Optional.
/// * `alias`: A user-defined shorthand for a full or partial URL. Optional.
///   - An alias must include at least a full domain but can be as long as desired.
///   - Example: 'work' could be an alias for 'github.com/some-super-long-organization-name'
/// * `frag`: A URL fragment that follows the alias, completing the URL if the alias is partial. Optional.
/// * `atom`: The atom id specced in the TOML manifest as `atom.id`.
/// * `version`: The version of the atom. Optional.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(test, derive(Serialize, Deserialize))]
struct Ref<'a> {
    /// The URI scheme (e.g., "https", "ssh"), if present.
    scheme: Option<&'a str>,
    user: Option<&'a str>,
    pass: Option<&'a str>,
    /// An alias for a full or partial URL, if present.
    alias: Option<&'a str>,
    /// A URL fragment that completes the URL when used with a partial alias, if present.
    frag: Option<&'a str>,
    /// The path to the specific atom within the repository.
    atom: Option<&'a str>,
    /// The version of the atom, if specified.
    version: Option<&'a str>,
}

impl<'a> From<&'a str> for Ref<'a> {
    /// Parses a string slice into a `Ref`.
    ///
    /// This is the primary way to create a `Ref` instance.
    ///
    /// # Arguments
    ///
    /// * `input` - A string slice containing the URI to parse.
    ///
    /// # Returns
    ///
    /// A `Ref` instance representing the parsed URI.
    fn from(input: &'a str) -> Self {
        use nom::{
            branch::alt,
            bytes::complete::{tag, take_until, take_while},
            character::complete::digit1,
            combinator::{map, not, opt, peek, recognize, rest, verify},
            sequence::tuple,
            IResult,
        };

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

        let user_pass = |input: &'a str| match recognize(verify(
            tuple((
                opt(peek(take_until(":"))),
                take_until("@"),
                tag("@"),
                peek(rest),
            )),
            |(_, q, _, a)| !(q as &str).contains('/') && VersionReq::parse(a as &str).is_err(),
        ))(input)
        {
            Ok((r, i)) => map(
                tuple((
                    opt(tuple((take_until(":"), tag(":")))),
                    opt(tuple((take_until("@"), tag("@")))),
                )),
                |(user, pass)| match (user, pass) {
                    (_, None) => (None, None),
                    (None, Some((u, _))) => (Some(u), None),
                    (Some((u, _)), Some((p, _))) => (Some(u), Some(p)),
                },
            )(i)
            .map(|(_, i)| (r, i)),
            Err(e) => Err(e),
        };

        let alias = |input: &'a str| {
            opt(map(
                verify(
                    tuple((
                        take_until(":"),
                        tag(":"),
                        peek(not(alt((
                            // not an atom
                            tag(":"),
                            // not a port
                            digit1,
                        )))),
                    )),
                    |(b, _, _)| {
                        // not an SSH-style url
                        <str>::find(b as &str, |c| c == '.').is_none()
                    },
                ),
                |(alias, _, _)| alias,
            ))(input)
            .map(empty)
        };

        let frag = |input: &'a str| {
            opt(map(tuple((take_until("::"), tag("::"))), |(frag, _)| frag))(input).map(empty)
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

        let (_, (scheme, maybe, alias, frag, atom, version)) =
            tuple((scheme, opt(user_pass), alias, frag, atom, version))(input).unwrap_or_default();

        let (user, pass) = maybe.unwrap_or((None, None));

        let (alias, frag) = match (alias, frag) {
            (None, Some(f)) if !f.contains('/') && !f.contains('.') => (Some(f), None),
            _ => (alias, frag),
        };

        tracing::trace!(scheme, user, pass, alias, frag, atom, version);

        Ref {
            scheme,
            user,
            pass,
            alias,
            frag,
            atom,
            version,
        }
    }
}

#[derive(Error, Debug)]
pub enum UriError {
    #[error(transparent)]
    AliasValidation(#[from] IdError),
    #[error(transparent)]
    InvalidVersionReq(#[from] semver::Error),
    #[error(transparent)]
    UrlParse(#[from] gix::url::parse::Error),
    #[error("The passed alias does not exist: {0}")]
    NoAlias(String),
    #[error("Missing ID in atom URI: [scheme://][alias:][url-fragment::]atom-id[@version]")]
    NoAtom,
}

use std::borrow::Cow;
impl<'a> Aliases {
    fn get_alias(&self, s: &str) -> Result<&str, UriError> {
        self.get(s)
            .map_or_else(|| Err(UriError::NoAlias(s.into())), |s| Ok(*s))
    }

    fn resolve_alias(&'a self, s: &str) -> Result<Cow<'a, str>, UriError> {
        let res = self.get_alias(s)?;

        // allow one level of indirection in alises, e.g. `org = gh:my-org`
        let res = match res.split_once(':') {
            Some((s, rest)) => {
                let res = self.get_alias(s)?;
                Cow::Owned(format!("{}:{}", res, rest))
            }
            None => Cow::Borrowed(res),
        };

        Ok(res)
    }
}

impl Deref for Aliases {
    type Target = HashMap<&'static str, &'static str>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Ref<'a> {
    fn delimited_alias(&self, aliases: &Aliases) -> Option<String> {
        let alias = self.alias.unwrap_or("");
        let resolved = aliases.resolve_alias(alias).unwrap_or(Cow::from(alias));
        let delim = match self.scheme {
            Some("ssh") => ":",
            Some(_) => "/",
            None => {
                if alias.is_empty() || self.frag.is_none() {
                    ""
                } else if alias == resolved {
                    ":"
                } else {
                    match (self.user, self.pass) {
                        (Some(_), None) => ":",
                        _ => "/",
                    }
                }
            }
        };
        self.alias.map(|_| format!("{}{}", resolved, delim))
    }
}

impl<'a> TryFrom<Parser<'a>> for Uri {
    type Error = UriError;
    fn try_from(parser: Parser<'a>) -> Result<Self, Self::Error> {
        let Parser { aliases, refs } = parser;
        let Ref {
            scheme,
            user,
            pass,
            alias: _,
            frag,
            atom,
            version,
        } = refs;

        let scheme = match scheme {
            Some("ssh") => "".into(),
            Some(s) => format!("{}://", s),
            None => if user.is_some() && pass.is_some() {
                "https://"
            } else {
                ""
            }
            .into(),
        };

        let start = match (user, pass) {
            (None, _) => scheme,
            (Some(u), None) => {
                format!("{}{}@", scheme, u)
            }
            (Some(u), Some(p)) => {
                format!("{}{}:{}@", scheme, u, p)
            }
        };

        let resolved = refs.delimited_alias(&aliases);
        let url_string: Option<String> = match (resolved, frag) {
            (None, None) => None,
            (None, Some(f)) => format!("{}{}", start, f).into(),
            (Some(r), None) => format!("{}{}", start, r).into(),
            (Some(r), Some(f)) => format!("{}{}{}", start, r, f).into(),
        };

        use gix::url::Scheme;

        let url: Option<Url> = match &url_string.clone().map(TryInto::try_into).transpose()? {
            Some(
                url @ Url {
                    scheme: Scheme::File,
                    path,
                    ..
                },
            ) => {
                let path = path.to_string();
                if path
                    .split_once('/')
                    .and_then(|(domain, _)| addr::parse_dns_name(domain).ok())
                    .or_else(|| addr::parse_dns_name(&path).ok())
                    .filter(|domain| domain.has_known_suffix() && domain.as_str().contains('.'))
                    .is_some()
                {
                    Some(format!("https://{}", &url_string.unwrap()).try_into()?)
                } else {
                    Some(url.to_owned())
                }
            }
            p => p.to_owned(),
        };

        let id = Id::try_from(atom.ok_or(UriError::NoAtom)?)?;
        let version = if let Some(v) = version {
            VersionReq::parse(v)?.into()
        } else {
            None
        };

        Ok(Uri { url, id, version })
    }
}

impl<'a> From<Ref<'a>> for Parser<'a> {
    fn from(r: Ref<'a>) -> Self {
        use config::CONFIG;

        let aliases = CONFIG.aliases();
        let aliases = Aliases(aliases.to_owned());
        Parser { aliases, refs: r }
    }
}

impl FromStr for Uri {
    type Err = UriError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let r = Ref::from(s);
        let p = Parser::from(r);
        Uri::try_from(p)
    }
}

impl<'a> TryFrom<&'a str> for Uri {
    type Error = UriError;
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        s.parse()
    }
}
