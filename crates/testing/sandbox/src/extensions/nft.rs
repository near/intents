use near_contract_standards::non_fungible_token::{Token, metadata::NFTContractMetadata};
use near_kit::{
    AccountIdRef, Gas, NearToken, SigningNear,
    protocol::{Action, FunctionCallAction},
    standards::{NftContractMetadata, NonFungibleToken, NonFungibleTokenClient},
};
use serde_json::json;

use crate::account::Account;

pub trait NftAdminExt {
    async fn deploy_vanilla_nft_issuer(
        &self,
        token_name: impl AsRef<str>,
        owner_id: impl AsRef<AccountIdRef>,
        metadata: &NFTContractMetadata,
        wasm: impl Into<Vec<u8>>,
    ) -> NonFungibleTokenClient<SigningNear>;

    async fn mint_nft(
        &self,
        collection: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
        token_owner_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<Token>;
}

impl NftAdminExt for SigningNear {
    async fn deploy_vanilla_nft_issuer(
        &self,
        token_name: impl AsRef<str>,
        owner_id: impl AsRef<AccountIdRef>,
        metadata: &NFTContractMetadata,
        wasm: impl Into<Vec<u8>>,
    ) -> NonFungibleTokenClient<Self> {
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
                    gas: Gas::from_tgas(30),
                    deposit: NearToken::from_near(0),
                }),
            )
            .await
            .unwrap();

        self.contract::<NonFungibleToken>(account.account_id())
    }

    async fn mint_nft(
        &self,
        collection: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
        token_owner_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<Token> {
        self.transaction(collection.as_ref())
            .add_action(Action::FunctionCall(FunctionCallAction {
                method_name: "nft_mint".to_string(),
                args: serde_json::to_vec(&json!({
                    "token_id": token_id.as_ref(),
                    "token_owner_id": token_owner_id.as_ref(),
                    "token_metadata": NftContractMetadata {
                        spec: String::default(),
                        name: String::default(),
                        symbol: String::default(),
                        icon: None,
                        base_uri: None,
                        reference: None,
                        reference_hash: None
                    },
                }))
                .unwrap(),
                gas: Gas::from_tgas(50),
                deposit: NearToken::from_near(1),
            }))
            .await?
            .json()
            .map_err(Into::into)
    }
}
