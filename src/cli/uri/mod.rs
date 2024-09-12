use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    character::complete::digit1,
    combinator::{map, not, opt, peek, recognize, verify},
    sequence::tuple,
    IResult,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
use serde::{Deserialize, Serialize};

/// Represents the parsed components of an atom reference URI.
///
/// This struct is an intermediate representation resulting from parsing a URI string
/// in the format: `[scheme://][alias:[url-fragment//]]atom-path[@version]`
///
/// It is typically created through the `From<&str>` implementation, not constructed directly.
///
/// # Components
///
/// * `scheme`: The URI scheme (e.g., "https", "ssh"). Optional.
/// * `alias`: A user-defined shorthand for a full or partial URL. Optional.
///   - An alias must include at least a full domain but can be as long as desired.
///   - Example: 'work' could be an alias for 'github.com/some-super-long-organization-name'
/// * `frag`: A URL fragment that follows the alias, completing the URL if the alias is partial. Optional.
/// * `atom`: The path to the specific atom within the given or local repository.
/// * `version`: The version of the atom. Optional.
///
/// # Examples
///
/// Parsing a full URI with an alias:
/// ```
/// use eka::cli::uri::Ref;
///
/// let uri_str = "https://work:our-repo//path/to/atom@1.0.0";
/// let uri_ref: Ref = uri_str.into();
///
/// assert_eq!(uri_ref.scheme(), Some("https"));
/// assert_eq!(uri_ref.alias(), Some("work"));
/// assert_eq!(uri_ref.frag(), Some("our-repo"));
/// assert_eq!(uri_ref.atom(), Some("path/to/atom"));
/// assert_eq!(uri_ref.version(), Some("1.0.0"));
/// ```
///
/// Parsing a URI with just an alias and atom path:
/// ```
/// use eka::cli::uri::Ref;
///
/// let uri_str = "work:our-repo//path/to/atom";
/// let uri_ref: Ref = uri_str.into();
///
/// assert_eq!(uri_ref.scheme(), None);
/// assert_eq!(uri_ref.alias(), Some("work"));
/// assert_eq!(uri_ref.frag(), Some("our-repo"));
/// assert_eq!(uri_ref.atom(), Some("path/to/atom"));
/// assert_eq!(uri_ref.version(), None);
/// ```
///
/// Parsing a minimal URI (only atom path):
/// ```
/// use eka::cli::uri::Ref;
///
/// let uri_str = "path/to/atom";
/// let uri_ref: Ref = uri_str.into();
///
/// assert_eq!(uri_ref.scheme(), None);
/// assert_eq!(uri_ref.alias(), None);
/// assert_eq!(uri_ref.frag(), None);
/// assert_eq!(uri_ref.atom(), Some("path/to/atom"));
/// assert_eq!(uri_ref.version(), None);
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(test, derive(Serialize, Deserialize))]
pub struct Ref<'a> {
    /// The URI scheme (e.g., "https", "ssh"), if present.
    scheme: Option<&'a str>,
    /// An alias for a full or partial URL, if present.
    alias: Option<&'a str>,
    /// A URL fragment that completes the URL when used with a partial alias, if present.
    frag: Option<&'a str>,
    /// The path to the specific atom within the repository.
    atom: Option<&'a str>,
    /// The version of the atom, if specified.
    version: Option<&'a str>,
}

impl<'a> Ref<'a> {
    pub fn scheme(&self) -> Option<&'a str> {
        self.scheme
    }
    pub fn alias(&self) -> Option<&'a str> {
        self.alias
    }
    pub fn frag(&self) -> Option<&'a str> {
        self.frag
    }
    pub fn atom(&self) -> Option<&'a str> {
        self.atom
    }
    pub fn version(&self) -> Option<&'a str> {
        self.version
    }
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

        Ref {
            scheme,
            alias,
            frag,
            atom,
            version,
        }
    }
}
