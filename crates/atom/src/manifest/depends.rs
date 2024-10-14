use std::path::PathBuf;

use semver::Version;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
enum AtomSrc {
    Url(Url),
    Path(PathBuf),
}

/// atom dependencies
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Dependencies {
    version: Version,
    src: AtomSrc,
}

/// legacy pins
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Pins {
    url: Url,
}

/// sources fetched at build time
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Srcs {
    url: Url,
}

impl AtomSrc {
    pub(crate) fn url(self) -> Option<Url> {
        match self {
            AtomSrc::Url(url) => Some(url),
            _ => None,
        }
    }

    pub(crate) fn path(self) -> Option<PathBuf> {
        match self {
            AtomSrc::Path(path) => Some(path),
            _ => None,
        }
    }
}
