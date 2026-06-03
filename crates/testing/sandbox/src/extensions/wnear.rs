use anyhow::Result;
use near_kit::{FunctionCallAction, NearToken};

use crate::{DEFAULT_GAS, Sandbox};

#[near_kit::contract]
pub trait WNear {
    #[call]
    fn near_deposit(&mut self, amount: NearToken);

    #[call]
    fn near_withdraw(&mut self, amount: NearToken);
}

impl Sandbox {
    pub async fn deploy_wrap_near(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> Result<WNearClient> {
        let contract_id = self
            .deploy_sub_contract(
                name,
                NearToken::from_near(100),
                wasm,
                Some(FunctionCallAction {
                    method_name: "new".to_string(),
                    args: vec![],
                    gas: DEFAULT_GAS,
                    deposit: NearToken::from_near(0),
                }),
            )
            .await?;

        Ok(self.contract::<dyn WNear>(contract_id))
    }
}
