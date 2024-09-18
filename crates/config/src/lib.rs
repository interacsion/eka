#[cfg(feature = "git")]
use gix::ThreadSafeRepository;

use etcetera::BaseStrategy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use figment::{
    providers::{Env, Format, Toml},
    Figment, Metadata, Provider,
};

#[derive(Deserialize, Serialize)]
pub struct Config {
    aliases: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            aliases: HashMap::from_iter([
                ("gh".into(), "github.com".into()),
                ("gl".into(), "gitlab.com".into()),
                ("bb".into(), "bitbucket.org".into()),
                ("sf".into(), "git.code.sf.net".into()),
            ]),
        }
    }
}

impl Config {
    pub fn from<T: Provider>(provider: T) -> Result<Config, figment::Error> {
        Figment::from(provider).extract()
    }

    pub fn figment() -> Figment {
        let mut fig = Figment::from(Config::default());

        if let Ok(c) = etcetera::choose_base_strategy() {
            let config = c.config_dir().join("eka.toml");
            fig = fig.admerge(Toml::file(config));
        }

        #[cfg(feature = "git")]
        if let Ok(r) = ThreadSafeRepository::discover(".") {
            let repo_config = r.git_dir().join("info/eka.toml");
            fig = fig.admerge(Toml::file(repo_config));
        };

        fig.admerge(Env::prefixed("EKA_"))
    }
}

impl Provider for Config {
    fn metadata(&self) -> figment::Metadata {
        Metadata::named("Eka CLI Config")
    }
    fn data(
        &self,
    ) -> Result<figment::value::Map<figment::Profile, figment::value::Dict>, figment::Error> {
        figment::providers::Serialized::defaults(Config::default()).data()
    }
}
