use near_contract_standards::non_fungible_token::{Token, TokenId, metadata::TokenMetadata};
use near_sdk::{AccountIdRef, Gas, NearToken, serde_json::json};

use crate::{Account, SigningAccount, TxResult};

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
            .function_call_json(
                "nft_transfer",
                json!({
                    "receiver_id": receiver_id,
                    "token_id": token_id,
                    "memo": memo,
                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(1),
            )
            .await
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
            .function_call_json(
                "nft_transfer_call",
                json!({
                    "receiver_id": receiver_id,
                    "token_id": token_id,
                    "memo": memo,
                    "msg": msg,

                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(1),
            )
            .await
    }

    async fn nft_mint(
        &self,
        collection: &AccountIdRef,
        token_id: &TokenId,
        token_owner_id: &AccountIdRef,
        token_metadata: &TokenMetadata,
    ) -> TxResult<Token> {
        self.tx(collection.into())
            .function_call_json(
                "nft_mint",
                json!({
                    "token_id": token_id,
                    "token_owner_id": token_owner_id,
                    "token_metadata": token_metadata,

                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(1),
            )
            .await
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
