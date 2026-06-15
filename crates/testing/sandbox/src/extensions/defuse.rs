use anyhow::Result;
use defuse::contract::config::DefuseConfig;
use defuse_core::{Nonce, PublicKey, fees::Pips};
use defuse_serde_utils::base64::AsBase64;
use near_kit::{Action, Final, FunctionCallAction, Near, NearToken};
use near_sdk::{
    AccountId,
    json_types::U128,
    serde::{Deserialize, Serialize},
    serde_json::json,
};
use std::collections::HashSet;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MtBatchBalanceOfArgs {
    pub account_id: AccountId,
    pub token_ids: Vec<String>,
}

use crate::{account::Account, extensions::DEFAULT_GAS, outcome::SuccessfulExecutionOutcome};

pub use defuse::contract;
pub use defuse::core;
pub use defuse::tokens;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct HasPublicKeyArgs {
    pub account_id: AccountId,
    pub public_key: PublicKey,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct IsNonceUsedArgs {
    pub account_id: AccountId,
    pub nonce: AsBase64<Nonce>,
}

#[near_kit::contract]
pub trait Defuse {
    fn fee(&self) -> Pips;
    fn fee_collector(&self) -> AccountId;

    fn has_public_key(&self, args: HasPublicKeyArgs) -> bool;
    fn public_keys_of(&self, account_id: AccountId) -> HashSet<PublicKey>;
    fn is_nonce_used(&self, args: IsNonceUsedArgs) -> bool;
    fn is_auth_by_predecessor_id_enabled(&self, account_id: AccountId) -> bool;
    fn is_account_locked(&self, account_id: AccountId) -> bool;
    fn mt_batch_balance_of(&self, args: MtBatchBalanceOfArgs) -> Vec<U128>;

    #[call]
    fn add_public_key(&mut self, public_key: PublicKey);

    #[call]
    fn remove_public_key(&mut self, public_key: PublicKey);

    #[call]
    fn disable_auth_by_predecessor_id(&mut self);

    #[call]
    fn set_fee(&mut self, fee: Pips);

    #[call]
    fn set_fee_collector(&mut self, fee_collector: AccountId);
}

pub trait DefuseExt {
    async fn defuse_add_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: PublicKey,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_remove_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: PublicKey,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_disable_auth_by_predecessor_id(
        &self,
        defuse: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_set_fee(
        &self,
        defuse: impl Into<AccountId>,
        fee: Pips,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn defuse_set_fee_collector(
        &self,
        defuse: impl Into<AccountId>,
        fee_collector: AccountId,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl DefuseExt for Near {
    async fn defuse_add_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: PublicKey,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(Defuse::add_public_key(public_key).gas(DEFAULT_GAS))
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_remove_public_key(
        &self,
        defuse: impl Into<AccountId>,
        public_key: PublicKey,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(Defuse::remove_public_key(public_key).gas(DEFAULT_GAS))
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_disable_auth_by_predecessor_id(
        &self,
        defuse: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(Defuse::disable_auth_by_predecessor_id().gas(DEFAULT_GAS))
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_set_fee(
        &self,
        defuse: impl Into<AccountId>,
        fee: Pips,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(Defuse::set_fee(fee).gas(DEFAULT_GAS))
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn defuse_set_fee_collector(
        &self,
        defuse: impl Into<AccountId>,
        fee_collector: AccountId,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(defuse.into())
            .add_action(Defuse::set_fee_collector(fee_collector).gas(DEFAULT_GAS))
            .wait_until(Final)
            .await?
            .try_into()
    }
}

pub trait DefuseDeployerExt {
    async fn deploy_defuse(
        &self,
        name: impl AsRef<str>,
        config: DefuseConfig,
        wasm: impl Into<Vec<u8>>,
    ) -> DefuseClient;
}

impl DefuseDeployerExt for Near {
    async fn deploy_defuse(
        &self,
        name: impl AsRef<str>,
        config: DefuseConfig,
        wasm: impl Into<Vec<u8>>,
    ) -> DefuseClient {
        let account = self
            .create_subaccount(name, NearToken::from_near(100))
            .await;

        let action = FunctionCallAction {
            method_name: "new".to_string(),
            args: json!({"config" : config}).to_string().as_bytes().to_vec(),
            gas: DEFAULT_GAS,
            deposit: NearToken::from_near(0),
        };

        account
            .deploy(wasm)
            .add_action(Action::FunctionCall(action))
            .wait_until(Final)
            .await
            .unwrap()
            .result()
            .unwrap();

        self.contract::<Defuse>(account.account_id())
    }
}
