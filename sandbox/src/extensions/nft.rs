use near_contract_standards::non_fungible_token::{Token, TokenId, metadata::TokenMetadata};
use near_sdk::{AccountIdRef, NearToken, serde_json::json};

use crate::{
    Account, SigningAccount,
    tx::{FnCallBuilder, TxResult},
};

pub trait NftExt {
    async fn nft_transfer(
        &self,
        collection: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_id: TokenId,
        memo: Option<String>,
    ) -> TxResult<()>;

    async fn nft_transfer_call(
        &self,
        collection: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_id: TokenId,
        memo: Option<String>,
        msg: String,
    ) -> TxResult<bool>;

    async fn nft_mint(
        &self,
        collection: &AccountIdRef,
        token_id: &TokenId,
        token_owner_id: &AccountIdRef,
        token_metadata: &TokenMetadata,
    ) -> TxResult<Token>;
}

pub trait NftViewExt {
    async fn nft_token(
        &self,
        collection: &AccountIdRef,
        token_id: &TokenId,
    ) -> anyhow::Result<Option<Token>>;
}

impl NftExt for SigningAccount {
    async fn nft_transfer(
        &self,
        collection: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_id: TokenId,
        memo: Option<String>,
    ) -> TxResult<()> {
        self.tx(collection.into())
            .function_call(
                FnCallBuilder::new("nft_transfer")
                    .json_args(&json!({
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
    ) -> TxResult<bool> {
        self.tx(collection.into())
            .function_call(
                FnCallBuilder::new("nft_transfer_call")
                    .json_args(&json!({
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
    ) -> TxResult<Token> {
        self.tx(collection.into())
            .function_call(
                FnCallBuilder::new("nft_mint")
                    .json_args(&json!({
                            "token_id": token_id,
                            "token_owner_id": token_owner_id,
                            "token_metadata": token_metadata,

                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }
}

impl NftViewExt for Account {
    async fn nft_token(
        &self,
        collection: &AccountIdRef,
        token_id: &TokenId,
    ) -> anyhow::Result<Option<Token>> {
        let account = Account::new(collection.into(), self.network_config().clone());

        account
            .call_view_function_json(
                "nft_token",
                json!({
                    "token_id": token_id
                }),
            )
            .await
    }
}

impl NftViewExt for SigningAccount {
    async fn nft_token(
        &self,
        collection: &AccountIdRef,
        token_id: &TokenId,
    ) -> anyhow::Result<Option<Token>> {
        self.account().nft_token(collection, token_id).await
    }
}
