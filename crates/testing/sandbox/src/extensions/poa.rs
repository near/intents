use anyhow::Result;
pub use defuse_poa_factory as contract;

use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_kit::{AccountId, FunctionCallAction, FungibleToken, Gas, NearToken, NonFungibleToken};
use near_sdk::{
    json_types::U128,
    serde::{Deserialize, Serialize},
};
use serde_json::json;
use std::collections::{HashMap, HashSet};

use crate::{DEFAULT_DEPOSIT, Sandbox, SubAcount};

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PoaDeployTokenArgs {
    pub token: String,
    pub metadata: Option<FungibleTokenMetadata>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PoaSetMetadataArgs {
    pub token: String,
    pub metadata: FungibleTokenMetadata,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
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

impl Sandbox {
    pub async fn deploy_poa_factory(
        &self,
        name: impl AsRef<str>,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<
            Item = (
                defuse_poa_factory::contract::Role,
                impl IntoIterator<Item = AccountId>,
            ),
        >,
        grantees: impl IntoIterator<
            Item = (
                defuse_poa_factory::contract::Role,
                impl IntoIterator<Item = AccountId>,
            ),
        >,
        wasm: impl Into<Vec<u8>>,
    ) -> Result<PoaFactoryClient<'_>> {
        let signer = self.deploy_sub_contract(
            name,
            NearToken::from_near(100),
            wasm,
            Some(FunctionCallAction {
                method_name: "new".to_string(),
                args: json!({
                    "super_admins": super_admins.into_iter().collect::<HashSet<_>>(),
                    "admins": admins
                        .into_iter()
                        .map(|(role, admins)| (role, admins.into_iter().collect::<HashSet<_>>()))
                        .collect::<HashMap<_, _>>(),
                    "grantees": grantees
                        .into_iter()
                        .map(|(role, grantees)| (role, grantees.into_iter().collect::<HashSet<_>>()))
                        .collect::<HashMap<_, _>>(),
        })
                .to_string()
                .into_bytes(),
                gas: Gas::DEFAULT,
                deposit: DEFAULT_DEPOSIT,
            })).await?;

        // TODO: this wold not work because of &Near
        Ok(PoaFactoryClient::new(
            &self,
            signer.account_id().unwrap().clone(),
        ))
    }

    pub async fn deploy_ft(
        &self,
        factory: &PoaFactoryClient<'_>,
        token: impl AsRef<str>,
    ) -> anyhow::Result<FungibleToken> {
        factory
            .deploy_token(PoaDeployTokenArgs {
                token: token.as_ref().to_string(),
                metadata: None,
            })
            .await?;

        let token_id = factory.contract_id().sub_account(token.as_ref())?;

        self.ft(token_id).map_err(Into::into)
    }

    pub async fn deploy_nft(
        &self,
        factory: &PoaFactoryClient<'_>,
        token: impl AsRef<str>,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> anyhow::Result<NonFungibleToken> {
        factory
            .deploy_token(PoaDeployTokenArgs {
                token: token.as_ref().to_string(),
                metadata: metadata.into(),
            })
            .await?;

        let token_id = factory.contract_id().sub_account(token.as_ref())?;

        self.nft(token_id).map_err(Into::into)
    }
}
