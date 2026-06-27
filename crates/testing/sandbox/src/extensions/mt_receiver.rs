use anyhow::Result;
use near_kit::{GlobalContractId, Near, NearToken};

use crate::global_contract::GlobalContract;

#[near_kit::contract]
pub trait MtReceiverStub {
    fn dummy_method(&self);
}

pub trait MtReceiverStubDeployerExt {
    async fn deploy_mt_receiver_stub_global(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> Result<GlobalContractId>;
}

impl MtReceiverStubDeployerExt for Near {
    async fn deploy_mt_receiver_stub_global(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> Result<GlobalContractId> {
        let account_id = self.account_id().sub_account(name.as_ref())?;

        self.deploy_upgradable_global_contract(&account_id, wasm, NearToken::from_near(40))
            .await?;

        Ok(GlobalContractId::AccountId(account_id))
    }
}
