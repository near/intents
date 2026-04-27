use anyhow::Context as _;
use config::Environment;
use defuse_outlayer_host::{Context, InMemorySigner, State};
use defuse_outlayer_primitives::{AccountIdRef, AppId};
use serde::Deserialize;
use serde_with::{hex::Hex, serde_as};
use std::borrow::Cow;

#[serde_as]
#[derive(Deserialize)]
pub struct TestConfig {
    #[serde(default = "default_app_id")]
    app_id: AppId<'static>,

    #[serde_as(as = "Hex")]
    #[serde(default = "default_seed")]
    seed: Vec<u8>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            app_id: default_app_id(),
            seed: default_seed(),
        }
    }
}

impl TestConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        config::Config::builder()
            .add_source(Environment::with_prefix("TEST_OUTLAYER"))
            .build()
            .context("build")?
            .try_deserialize()
            .context("deserialize")
    }

    pub fn build(self) -> State<'static> {
        State::new(
            Context {
                app_id: self.app_id,
            },
            Cow::Owned(InMemorySigner::from_seed(&self.seed)),
        )
    }
}

fn default_app_id() -> AppId<'static> {
    AccountIdRef::new_or_panic("0s7e579fce76b37e4e93b7605022da52e6ccc26fd2").into()
}

fn default_seed() -> Vec<u8> {
    b"test".to_vec()
}
