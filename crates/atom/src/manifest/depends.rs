use std::path::PathBuf;

use semver::VersionReq;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(
    rename_all = "lowercase",
    expecting = "requires only a `path` or `url` key with optional `ref`"
)]
enum Src {
    Path(PathBuf),
    #[serde(untagged)]
    Url {
        url: Url,
        #[cfg(feature = "git")]
        #[serde(
            skip_serializing_if = "Option::is_none",
            serialize_with = "serialize_ref"
        )]
        r#ref: Option<gix::refs::PartialName>,
    },
}

/// atom dependencies
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Atoms {
    version: VersionReq,
    #[serde(flatten)]
    src: Src,
}

/// legacy pins and buildtime srcs. We use a single type to
/// represent both as they share the same form.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Srcs {
    #[serde(flatten)]
    src: Src,
}

#[allow(dead_code)]
impl Src {
    pub(crate) fn url(self) -> Option<Url> {
        match self {
            Src::Url { url, .. } => Some(url),
            _ => None,
        }
    }

    pub(crate) fn path(self) -> Option<PathBuf> {
        match self {
            Src::Path(path) => Some(path),
            _ => None,
        }
    }
}

#[cfg(feature = "git")]
fn serialize_ref<S>(
    bytes: &Option<gix::refs::PartialName>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    // calling unwrap is safe since we skip serialize on none
    let str = bytes.as_ref().unwrap().as_ref().as_bstr().to_owned();
    serializer.serialize_str(&str.to_string())
}
