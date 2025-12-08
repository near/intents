use near_api::types::nft::NFTContractMetadata;
use near_contract_standards::non_fungible_token::{Token, TokenId, metadata::TokenMetadata};
use near_sdk::{AccountIdRef, NearToken, serde_json::json};

use crate::{Account, SigningAccount, extensions::account::AccountDeployerExt, tx::FnCallBuilder};

// TODO: make it prettier
const NON_FUNGIBLE_TOKEN_WASM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../releases/non-fungible-token.wasm"
));

#[allow(async_fn_in_trait)]
pub trait NftExt {
    async fn nft_transfer(
        &self,
        collection: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_id: TokenId,
        memo: Option<String>,
    ) -> anyhow::Result<()>;

    async fn nft_transfer_call(
        &self,
        collection: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_id: TokenId,
        memo: Option<String>,
        msg: String,
    ) -> anyhow::Result<bool>;

    async fn nft_mint(
        &self,
        collection: &AccountIdRef,
        token_id: &TokenId,
        token_owner_id: &AccountIdRef,
        token_metadata: &TokenMetadata,
    ) -> anyhow::Result<Token>;
}

#[allow(async_fn_in_trait)]
pub trait NftDeployerExt {
    async fn deploy_vanilla_nft_issuer(
        &self,
        token_name: &str,
        metadata: NFTContractMetadata,
    ) -> anyhow::Result<Account>;
}

#[allow(async_fn_in_trait)]
pub trait NftViewExt {
    async fn nft_token(&self, token_id: &TokenId) -> anyhow::Result<Option<Token>>;
}

impl NftExt for SigningAccount {
    async fn nft_transfer(
        &self,
        collection: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_id: TokenId,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.tx(collection.into())
            .function_call(
                FnCallBuilder::new("nft_transfer")
                    .json_args(json!({
                        "receiver_id": receiver_id,
                        "token_id": token_id,
                        "memo": memo,
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        Ok(())
    }

    async fn nft_transfer_call(
        &self,
        collection: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_id: TokenId,
        memo: Option<String>,
        msg: String,
    ) -> anyhow::Result<bool> {
        self.tx(collection.into())
            .function_call(
                FnCallBuilder::new("nft_transfer_call")
                    .json_args(json!({
                            "receiver_id": receiver_id,
                            "token_id": token_id,
                            "memo": memo,
                        "msg": msg,

                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn nft_mint(
        &self,
        collection: &AccountIdRef,
        token_id: &TokenId,
        token_owner_id: &AccountIdRef,
        token_metadata: &TokenMetadata,
    ) -> anyhow::Result<Token> {
        self.tx(collection.into())
            .function_call(
                FnCallBuilder::new("nft_mint")
                    .json_args(json!({
                            "token_id": token_id,
                            "token_owner_id": token_owner_id,
                            "token_metadata": token_metadata,

                    }))
                    .with_deposit(NearToken::from_near(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }
}

impl NftDeployerExt for SigningAccount {
    async fn deploy_vanilla_nft_issuer(
        &self,
        token_name: &str,
        metadata: NFTContractMetadata,
    ) -> anyhow::Result<Account> {
        let args = json!({
            "owner_id": self.id(),
            "metadata": metadata
        });

        self.deploy_contract(
            token_name,
            NON_FUNGIBLE_TOKEN_WASM,
            Some(FnCallBuilder::new("new").json_args(&args)),
        )
        .await
    }
}

impl NftViewExt for Account {
    async fn nft_token(&self, token_id: &TokenId) -> anyhow::Result<Option<Token>> {
        self.call_view_function_json(
            "nft_token",
            json!({
                "token_id": token_id
            }),
        )
        .await
    }
}

impl NftViewExt for SigningAccount {
    async fn nft_token(&self, token_id: &TokenId) -> anyhow::Result<Option<Token>> {
        self.account().nft_token(token_id).await
    }
}
