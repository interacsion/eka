#[cfg(test)]
mod tests;

use super::core::Manifest;
use super::Name;
use semver::VersionReq;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Depend {
    #[serde(flatten)]
    core: Manifest,
    deps: HashMap<Name, Dependency>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Dependency {
    version: VersionReq,
    repo: Url,
}
