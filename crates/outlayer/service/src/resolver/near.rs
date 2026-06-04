use std::{sync::Arc, time::Duration};

use defuse_outlayer_primitives::account_id::AccountId;
use defuse_serde_utils::hex::AsHex;

use futures::try_join;
use moka::future::Cache;
use near_kit::Near;
use tracing::Instrument;
use url::Url;

use crate::AppCodeUrl;

#[derive(Clone)]
pub struct NearResolver {
    client: Near,
    cache: Cache<AccountId, AppCodeUrl>,
}

impl NearResolver {
    pub fn new(client: Near, cache_ttl: Duration) -> Self {
        Self {
            client,
            cache: Cache::builder().time_to_live(cache_ttl).build(),
        }
    }

    pub async fn resolve(
        &self,
        oa_contract_id: impl Into<AccountId>,
    ) -> Result<AppCodeUrl, Arc<near_kit::Error>> {
        let oa_contract_id = oa_contract_id.into();
        self.cache
            .try_get_with(oa_contract_id.clone(), self.fetch(oa_contract_id))
            .await
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn fetch(&self, oa_contract_id: AccountId) -> Result<AppCodeUrl, near_kit::Error> {
        let oa = self.client.contract::<OutlayerApp>(oa_contract_id);
        // TODO: finality or speicific block hash?
        let (code_url, code_hash) = try_join!(
            // TODO: limit the length of fetched data URLs?
            oa.oa_code_url().into_future(),
            oa.oa_code_hash().into_future(),
        )?;
        Ok(AppCodeUrl {
            code_url,
            code_hash: code_hash.0,
        })
    }
}

#[near_kit::contract]
trait OutlayerApp {
    fn oa_code_hash(&self) -> AsHex<[u8; 32]>;
    fn oa_code_url(&self) -> Url;
}
