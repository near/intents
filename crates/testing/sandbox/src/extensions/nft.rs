use near_contract_standards::non_fungible_token::{Token, metadata::NFTContractMetadata};
use near_kit::{
    AccountIdRef, Action, Final, FunctionCallAction, Near, NearToken, NonFungibleToken,
};
use serde_json::json;

use crate::{account::Account, extensions::DEFAULT_GAS};

pub trait NftAdminExt {
    async fn deploy_vanilla_nft_issuer(
        &self,
        token_name: impl AsRef<str>,
        owner_id: impl AsRef<AccountIdRef>,
        metadata: &NFTContractMetadata,
        wasm: impl Into<Vec<u8>>,
    ) -> NonFungibleToken;

    async fn mint_nft(
        &self,
        collection: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
        token_owner_id: impl AsRef<AccountIdRef>,
        token_metadata: &NFTContractMetadata,
    ) -> anyhow::Result<Token>;
}

impl NftAdminExt for Near {
    async fn deploy_vanilla_nft_issuer(
        &self,
        token_name: impl AsRef<str>,
        owner_id: impl AsRef<AccountIdRef>,
        metadata: &NFTContractMetadata,
        wasm: impl Into<Vec<u8>>,
    ) -> NonFungibleToken {
        let account = self
            .deploy_sub_contract(
                token_name,
                NearToken::from_near(100),
                wasm,
                Some(FunctionCallAction {
                    method_name: "new".to_string(),
                    args: serde_json::to_vec(&json!({
                        "owner_id": owner_id.as_ref(),
                        "metadata": metadata
                    }))
                    .unwrap(),
                    gas: DEFAULT_GAS,
                    deposit: NearToken::from_near(0),
                }),
            )
            .await
            .unwrap();

        self.nft(account.account_id()).unwrap()
    }

    async fn mint_nft(
        &self,
        collection: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
        token_owner_id: impl AsRef<AccountIdRef>,
        token_metadata: &NFTContractMetadata,
    ) -> anyhow::Result<Token> {
        self.transaction(collection.as_ref())
            .add_action(Action::FunctionCall(FunctionCallAction {
                method_name: "nft_mint".to_string(),
                args: serde_json::to_vec(&json!({
                    "token_id": token_id.as_ref(),
                    "token_owner_id": token_owner_id.as_ref(),
                    "token_metadata": token_metadata,
                }))
                .unwrap(),
                gas: DEFAULT_GAS,
                deposit: NearToken::from_near(1),
            }))
            .wait_until(Final)
            .await?
            .json()
            .map_err(Into::into)
    }
}
