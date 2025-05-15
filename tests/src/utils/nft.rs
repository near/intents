use super::account::AccountExt;
use near_contract_standards::non_fungible_token::{
    Token, TokenId,
    metadata::{NFTContractMetadata, TokenMetadata},
};
use near_sdk::{AccountId, NearToken};
use near_workspaces::Contract;
use serde_json::json;

const NON_FUNGIBLE_TOKEN_WASM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/non-fungible-token.wasm"
));

pub trait NftExt {
    async fn deploy_vanilla_nft_issuer(
        &self,
        token_name: &str,
        metadata: NFTContractMetadata,
    ) -> anyhow::Result<Contract>;

    async fn nft_transfer(
        &self,
        collection: &AccountId,
        receiver_id: &AccountId,
        token_id: TokenId,
        memo: Option<String>,
    ) -> anyhow::Result<()>;

    async fn nft_transfer_call(
        &self,
        collection: &AccountId,
        receiver_id: &AccountId,
        token_id: TokenId,
        memo: Option<String>,
        msg: String,
    ) -> anyhow::Result<bool>;

    async fn nft_mint(
        &self,
        collection: &AccountId,
        token_id: &TokenId,
        token_owner_id: &AccountId,
        token_metadata: &TokenMetadata,
    ) -> anyhow::Result<Token>;

    async fn nft_token(
        &self,
        collection: &AccountId,
        token_id: &TokenId,
    ) -> anyhow::Result<Option<Token>>;

    async fn self_nft_token(&self, token_id: &TokenId) -> anyhow::Result<Option<Token>>;
}

impl NftExt for near_workspaces::Account {
    async fn deploy_vanilla_nft_issuer(
        &self,
        token_name: &str,
        metadata: NFTContractMetadata,
    ) -> anyhow::Result<Contract> {
        let contract = self
            .deploy_contract(token_name, NON_FUNGIBLE_TOKEN_WASM)
            .await?;

        let args = json!({
            "owner_id": self.id(),
            "metadata": metadata
        });

        contract
            .call("new")
            .args_json(args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(contract)
    }
    async fn nft_transfer(
        &self,
        collection: &AccountId,
        receiver_id: &AccountId,
        token_id: TokenId,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.call(collection, "nft_transfer")
            .args_json(json!({
                "receiver_id": receiver_id,
                "token_id": token_id,
                "memo": memo,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn nft_transfer_call(
        &self,
        collection: &AccountId,
        receiver_id: &AccountId,
        token_id: TokenId,
        memo: Option<String>,
        msg: String,
    ) -> anyhow::Result<bool> {
        self.call(collection, "nft_transfer_call")
            .args_json(json!({
                "receiver_id": receiver_id,
                "token_id": token_id,
                "memo": memo,
                "msg": msg,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?
            .json()
            .map_err(Into::into)
    }

    async fn nft_mint(
        &self,
        collection: &AccountId,
        token_id: &TokenId,
        token_owner_id: &AccountId,
        token_metadata: &TokenMetadata,
    ) -> anyhow::Result<Token> {
        self.call(collection, "nft_mint")
            .args_json(json!({
                "token_id": token_id,
                "token_owner_id": token_owner_id,
                "token_metadata": token_metadata,
            }))
            .deposit(NearToken::from_near(1))
            .transact()
            .await?
            .into_result()?
            .json()
            .map_err(Into::into)
    }

    async fn nft_token(
        &self,
        collection: &AccountId,
        token_id: &TokenId,
    ) -> anyhow::Result<Option<Token>> {
        self.view(collection, "nft_token")
            .args_json(json!({
                "token_id": token_id,
            }))
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn self_nft_token(&self, token_id: &TokenId) -> anyhow::Result<Option<Token>> {
        self.nft_token(self.id(), token_id).await
    }
}

impl NftExt for Contract {
    async fn deploy_vanilla_nft_issuer(
        &self,
        token_name: &str,
        metadata: NFTContractMetadata,
    ) -> anyhow::Result<Contract> {
        self.as_account()
            .deploy_vanilla_nft_issuer(token_name, metadata)
            .await
    }

    async fn nft_transfer(
        &self,
        collection: &AccountId,
        receiver_id: &AccountId,
        token_id: TokenId,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.as_account()
            .nft_transfer(collection, receiver_id, token_id, memo)
            .await
    }

    async fn nft_transfer_call(
        &self,
        collection: &AccountId,
        receiver_id: &AccountId,
        token_id: TokenId,
        memo: Option<String>,
        msg: String,
    ) -> anyhow::Result<bool> {
        self.as_account()
            .nft_transfer_call(collection, receiver_id, token_id, memo, msg)
            .await
    }

    async fn nft_mint(
        &self,
        collection: &AccountId,
        token_id: &TokenId,
        token_owner_id: &AccountId,
        token_metadata: &TokenMetadata,
    ) -> anyhow::Result<Token> {
        self.as_account()
            .nft_mint(collection, token_id, token_owner_id, token_metadata)
            .await
    }

    async fn nft_token(
        &self,
        collection: &AccountId,
        token_id: &TokenId,
    ) -> anyhow::Result<Option<Token>> {
        self.as_account().nft_token(collection, token_id).await
    }

    async fn self_nft_token(&self, token_id: &TokenId) -> anyhow::Result<Option<Token>> {
        self.as_account().self_nft_token(token_id).await
    }
}
