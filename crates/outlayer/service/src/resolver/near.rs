use defuse_outlayer_primitives::account_id::AccountId;
use defuse_serde_utils::hex::AsHex;
use futures::try_join;
use near_kit::Near;
use url::Url;

use crate::AppCodeUrl;

pub struct NearResolver {
    client: Near,
}

impl NearResolver {
    pub async fn resolve(
        &self,
        oa_contract_id: impl Into<AccountId>,
    ) -> Result<AppCodeUrl, near_kit::Error> {
        let oa = self.client.contract::<OutlayerApp>(oa_contract_id.into());
        // TODO: finality or speicific block hash?
        let (code_url, code_hash) = try_join!(
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
