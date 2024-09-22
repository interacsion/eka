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

/// Represents the parsed components of an atom reference URI.
///
/// This struct is an intermediate representation resulting from parsing a URI string
/// in the format: `[scheme://][alias:][url-fragment::]atom-id[@version]`
///
/// It is typically created through the `FromStr` implementation, not constructed directly.
///
/// # Components
///
/// * `url`: URL to the repository containing the atom.
/// * `id`: The atom ID specced in the TOML manifest as `atom.id`.
/// * `version`: The requested atom version. Optional.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Uri {
    /// The URL to the repository containing the atom
    url: Option<Url>,
    /// The atom's ID
    id: Id,
    /// The requested atom version
    version: Option<VersionReq>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(test, derive(Serialize, Deserialize))]
struct Ref<'a> {
    #[cfg_attr(test, serde(borrow))]
    url: UrlRef<'a>,
    atom: AtomRef<'a>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(test, derive(Serialize, Deserialize))]
struct UrlRef<'a> {
    /// The URI scheme (e.g., "https", "ssh"), if present.
    scheme: Option<&'a str>,
    /// The username.
    user: Option<&'a str>,
    /// The password.
    pass: Option<&'a str>,
    /// A URL fragment which may contain an alias to be later expanded
    frag: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(test, derive(Serialize, Deserialize))]
struct AtomRef<'a> {
    /// The path to the specific atom within the repository.
    id: Option<&'a str>,
    /// The version of the atom, if specified.
    version: Option<&'a str>,
}

use nom::{
    bytes::complete::{tag, take_until},
    character::complete::digit1,
    combinator::{map, not, opt, peek, verify},
    sequence::tuple,
    IResult,
};

fn parse(input: &str) -> Ref {
    let (rest, url) = match url(input) {
        Ok(s) => s,
        Err(_) => (input, None),
    };

    let url = url.map(UrlRef::from).unwrap_or_default();

    let atom = AtomRef::from(rest);

    tracing::trace!(
        url.scheme,
        url.user,
        url.pass = url.pass.map(|_| "<redacted>"),
        url.frag,
        atom.id,
        atom.version,
        "{}",
        input
    );

    Ref { url, atom }
}

fn parse_alias(input: &str) -> IResult<&str, Option<&str>> {
    opt(map(
        verify(
            tuple((
                take_until(":"),
                tag(":"),
                // not a port
                peek(not(digit1)),
            )),
            // not an scp url
            |(a, _, _)| !(a as &str).contains('.'),
        ),
        |(alias, _, _)| alias,
    ))(input)
    .map(empty_none)
}

type UrlPrefix<'a> = (Option<&'a str>, Option<&'a str>, Option<&'a str>);

fn parse_url(url: &str) -> IResult<&str, UrlPrefix> {
    let (rest, (scheme, user_pass)) = tuple((scheme, split_at))(url)?;

    let (user, pass) = match user_pass {
        Some(s) => match split_colon(s) {
            Ok((p, Some(u))) => (Some(u), Some(p)),
            Ok((u, None)) => (Some(u), None),
            _ => (Some(s), None),
        },
        None => (None, None),
    };

    Ok((rest, (scheme, user, pass)))
}

fn not_empty(input: &str) -> Option<&str> {
    if input.is_empty() {
        None
    } else {
        Some(input)
    }
}

fn empty_none<'a>((rest, opt): (&'a str, Option<&'a str>)) -> (&'a str, Option<&'a str>) {
    (rest, opt.and_then(not_empty))
}

fn opt_split<'a>(input: &'a str, delim: &str) -> IResult<&'a str, Option<&'a str>> {
    opt(map(tuple((take_until(delim), tag(delim))), |(url, _)| url))(input).map(empty_none)
}

fn url(input: &str) -> IResult<&str, Option<&str>> {
    opt_split(input, "::")
}

fn scheme(input: &str) -> IResult<&str, Option<&str>> {
    opt_split(input, "://")
}

fn split_at(input: &str) -> IResult<&str, Option<&str>> {
    opt_split(input, "@")
}

fn split_colon(input: &str) -> IResult<&str, Option<&str>> {
    opt_split(input, ":")
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
        parse(input)
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
    #[error("Parsing URL failed")]
    NoUrl,
    #[error("Missing atom ID in URI")]
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

impl<'a> From<&'a str> for UrlRef<'a> {
    fn from(s: &'a str) -> Self {
        let (scheme, user, pass, frag) = match parse_url(s) {
            Ok((frag, (scheme, user, pass))) => (scheme, user, pass, not_empty(frag)),
            _ => (None, None, None, None),
        };

        Self {
            scheme,
            user,
            pass,
            frag,
        }
    }
}

impl<'a> From<&'a str> for AtomRef<'a> {
    fn from(s: &'a str) -> Self {
        let (id, version) = match split_at(s) {
            Ok((rest, Some(atom))) => (Some(atom), not_empty(rest)),
            Ok((rest, None)) => (not_empty(rest), None),
            _ => (None, None),
        };

        AtomRef { id, version }
    }
}

impl<'a> UrlRef<'a> {
    fn render_frag(&self) -> Option<String> {
        use config::CONFIG;

        let aliases = Aliases(CONFIG.aliases().to_owned());

        let (frag, alias) = match parse_alias(self.frag?) {
            Ok((f, Some(a))) => (Some(f), a),
            Ok((f, None)) => (None, f),
            Err(_) => (None, self.frag?),
        };

        let resolved = aliases.resolve_alias(alias).unwrap_or(Cow::from(alias));
        let delim = match self.scheme {
            Some("ssh") => ":",
            Some(_) => "/",
            None => {
                if frag.is_none() {
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
        tracing::trace!(alias, %resolved, delim, frag);
        Some(format!("{}{}{}", resolved, delim, frag.unwrap_or("")))
    }

    fn render_scheme(&self) -> Cow<'a, str> {
        match self.scheme {
            Some("ssh") => Cow::from(""),
            Some(s) => Cow::from(format!("{}://", s)),
            None => Cow::from(if self.user.is_some() && self.pass.is_some() {
                "https://"
            } else {
                ""
            }),
        }
    }

    fn render(&self) -> Option<String> {
        let scheme = self.render_scheme();
        self.render_frag().map(|frag| match (self.user, self.pass) {
            (None, _) => format!("{}{}", scheme, frag),
            (Some(u), None) => {
                format!("{}{}@{}", scheme, u, frag)
            }
            (Some(u), Some(p)) => {
                format!("{}{}:{}@{}", scheme, u, p, frag)
            }
        })
    }

    fn to_url(&self) -> Result<Option<Url>, UriError> {
        use gix::url::Scheme;

        let url_string = self.render();

        let url = match url_string.to_owned().map(TryInto::try_into).transpose()? {
            Some(
                url @ Url {
                    scheme: Scheme::File,
                    ..
                },
            ) => {
                let path = url.path.to_string();
                if path
                    .split_once('/')
                    .and_then(|(domain, _)| addr::parse_dns_name(domain).ok())
                    .or_else(|| addr::parse_dns_name(&path).ok())
                    .filter(|domain| domain.has_known_suffix() && domain.as_str().contains('.'))
                    .is_some()
                {
                    Some(format!("https://{}", &url_string.unwrap()).try_into()?)
                } else {
                    Some(url)
                }
            }
            p => p,
        };

        Ok(url)
    }
}

impl<'a> AtomRef<'a> {
    fn render(&self) -> Result<(Id, Option<VersionReq>), UriError> {
        let id = Id::try_from(self.id.ok_or(UriError::NoAtom)?)?;
        let version = if let Some(v) = self.version {
            VersionReq::parse(v)?.into()
        } else {
            None
        };
        Ok((id, version))
    }
}

impl<'a> TryFrom<Ref<'a>> for Uri {
    type Error = UriError;
    fn try_from(refs: Ref<'a>) -> Result<Self, Self::Error> {
        let Ref { url, atom } = refs;

        let url = url.to_url()?;

        let (id, version) = atom.render()?;

        Ok(Uri { url, id, version })
    }
}

impl<'a> TryFrom<UrlRef<'a>> for Url {
    type Error = UriError;
    fn try_from(refs: UrlRef<'a>) -> Result<Self, Self::Error> {
        match refs.to_url() {
            Ok(Some(url)) => Ok(url),
            Ok(None) => Err(UriError::NoUrl),
            Err(e) => Err(e),
        }
    }
}

impl FromStr for Uri {
    type Err = UriError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let r = Ref::from(s);
        Uri::try_from(r)
    }
}

impl<'a> TryFrom<&'a str> for Uri {
    type Error = UriError;
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl std::fmt::Display for Uri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let url = self.url.as_ref().map(|u| u.to_string()).unwrap_or_default();
        let version = self
            .version
            .as_ref()
            .map(|v| format!("@{}", v))
            .unwrap_or_default();
        write!(f, "{}::{}{}", &url.trim_end_matches('/'), self.id, &version)
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
