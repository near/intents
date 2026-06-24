use anyhow::Result;
use defuse_poa_factory::contract::Role;
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_kit::{
    AccountId, AccountIdRef, Final, FunctionCallAction, FungibleToken, Near, NearToken,
};
use near_sdk::json_types::U128;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};

use crate::{account::Account, extensions::DEFAULT_GAS, outcome::SuccessfulExecutionOutcome};

pub use defuse_poa_factory::contract;

pub const POA_TOKEN_INIT_BALANCE: NearToken = NearToken::from_near(3);

#[derive(Serialize, Deserialize)]
pub struct PoaDeployTokenArgs {
    pub token: String,
    pub metadata: Option<FungibleTokenMetadata>,
}

#[derive(Serialize, Deserialize)]
pub struct PoaSetMetadataArgs {
    pub token: String,
    pub metadata: FungibleTokenMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct PoaFtDepositArgs {
    pub token: String,
    pub owner_id: AccountId,
    pub amount: U128,
    pub msg: Option<String>,
    pub memo: Option<String>,
}

#[near_kit::contract]
pub trait PoaFactory {
    #[call]
    fn deploy_token(&mut self, args: PoaDeployTokenArgs);

    #[call]
    fn set_metadata(&mut self, args: PoaSetMetadataArgs);

    #[call]
    fn ft_deposit(&mut self, args: PoaFtDepositArgs);

    fn tokens(&self) -> HashMap<String, AccountId>;
}

impl PoaFactoryClient {
    pub fn ft_name(&self, ft: &AccountIdRef) -> String {
        ft.as_str()
            .trim_end_matches(&format!(".{}", self.contract_id()))
            .to_string()
    }
}

pub trait PoaFactoryDeployerExt {
    async fn deploy_poa_factory(
        &self,
        name: impl AsRef<str>,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        wasm: impl Into<Vec<u8>>,
    ) -> PoaFactoryClient;
}

impl PoaFactoryDeployerExt for Near {
    async fn deploy_poa_factory(
        &self,
        name: impl AsRef<str>,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        wasm: impl Into<Vec<u8>>,
    ) -> PoaFactoryClient {
        let action = FunctionCallAction {
            method_name: "new".to_string(),
            args: serde_json::to_vec(&json!({
                "super_admins": super_admins.into_iter().collect::<HashSet<_>>(),
                "admins": admins
                    .into_iter()
                    .map(|(role, admins)| (role, admins.into_iter().collect::<HashSet<_>>()))
                    .collect::<HashMap<_, _>>(),
                "grantees": grantees
                    .into_iter()
                    .map(|(role, grantees)| (role, grantees.into_iter().collect::<HashSet<_>>()))
                    .collect::<HashMap<_, _>>(),
            }))
            .unwrap(),
            gas: DEFAULT_GAS,
            deposit: NearToken::from_near(0),
        };

        let account = self
            .deploy_sub_contract(name, NearToken::from_near(10), wasm, Some(action))
            .await
            .unwrap();

        self.contract::<PoaFactory>(account.account_id())
    }
}

pub trait PoAFactoryExt {
    async fn poa_factory_deploy_token(
        &self,
        factory: impl AsRef<AccountIdRef>,
        token: impl AsRef<str>,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> Result<FungibleToken>;

    async fn poa_factory_ft_deposit(
        &self,
        factory: impl AsRef<AccountIdRef>,
        token: impl AsRef<str>,
        owner_id: impl AsRef<AccountIdRef>,
        amount: u128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl PoAFactoryExt for Near {
    async fn poa_factory_deploy_token(
        &self,
        factory: impl AsRef<AccountIdRef>,
        token: impl AsRef<str>,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> Result<FungibleToken> {
        let factory = factory.as_ref();
        self.transaction(factory)
            .add_action(
                PoaFactory::deploy_token(PoaDeployTokenArgs {
                    token: token.as_ref().to_string(),
                    metadata: metadata.into(),
                })
                .deposit(POA_TOKEN_INIT_BALANCE)
                .gas(DEFAULT_GAS),
            )
            .wait_until(Final)
            .await?
            .result()?;

        Ok(self.ft(factory.sub_account(token)?)?)
    }

    async fn poa_factory_ft_deposit(
        &self,
        factory: impl AsRef<AccountIdRef>,
        token: impl AsRef<str>,
        owner_id: impl AsRef<AccountIdRef>,
        amount: u128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(factory.as_ref())
            .add_action(
                PoaFactory::ft_deposit(PoaFtDepositArgs {
                    token: token.as_ref().to_string(),
                    owner_id: owner_id.as_ref().into(),
                    amount: amount.into(),
                    msg,
                    memo,
                })
                .deposit(NearToken::from_millinear(4))
                .gas(DEFAULT_GAS),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }
}
