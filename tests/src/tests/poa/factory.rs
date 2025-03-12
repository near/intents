use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use defuse_poa_factory::contract::Role;
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::{AccountId, NearToken, json_types::U128};
use near_workspaces::Contract;
use serde_json::json;

use crate::utils::{account::AccountExt, read_wasm};

static POA_FACTORY_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("defuse_poa_factory"));

pub trait PoAFactoryExt {
    async fn deploy_poa_factory(
        &self,
        name: &str,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
    ) -> anyhow::Result<Contract>;

    #[track_caller]
    fn token_id(token: &str, factory: &AccountId) -> AccountId {
        format!("{token}.{factory}").parse().unwrap()
    }
    async fn poa_factory_deploy_token(
        &self,
        factory: &AccountId,
        token: &str,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> anyhow::Result<AccountId>;
    async fn poa_deploy_token(
        &self,
        token: &str,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> anyhow::Result<AccountId>;

    async fn poa_factory_ft_deposit(
        &self,
        factory: &AccountId,
        token: &str,
        owner_id: &AccountId,
        amount: u128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> anyhow::Result<()>;
    async fn poa_ft_deposit(
        &self,
        token: &str,
        owner_id: &AccountId,
        amount: u128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> anyhow::Result<()>;
    async fn poa_factory_tokens(
        &self,
        poa_factory: &AccountId,
    ) -> anyhow::Result<HashMap<String, AccountId>>;
}

impl PoAFactoryExt for near_workspaces::Account {
    async fn deploy_poa_factory(
        &self,
        name: &str,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
    ) -> anyhow::Result<Contract> {
        let contract = self.deploy_contract(name, &POA_FACTORY_WASM).await?;
        self.transfer_near(contract.id(), NearToken::from_near(100))
            .await?
            .into_result()?;
        contract
            .call("new")
            .args_json(json!({
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
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(contract)
    }

    async fn poa_factory_deploy_token(
        &self,
        factory: &AccountId,
        token: &str,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> anyhow::Result<AccountId> {
        self.call(factory, "deploy_token")
            .args_json(json!({
                "token": token,
                "metadata": metadata.into(),
            }))
            .deposit(NearToken::from_near(4))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(Self::token_id(token, factory))
    }

    async fn poa_deploy_token(
        &self,
        token: &str,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> anyhow::Result<AccountId> {
        self.poa_factory_deploy_token(self.id(), token, metadata)
            .await
    }

    async fn poa_factory_ft_deposit(
        &self,
        factory: &AccountId,
        token: &str,
        owner_id: &AccountId,
        amount: u128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.call(factory, "ft_deposit")
            .args_json(json!({
                "token": token,
                "owner_id": owner_id,
                "amount": U128(amount),
                "msg": msg,
                "memo": memo,
            }))
            .deposit(NearToken::from_millinear(4))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn poa_ft_deposit(
        &self,
        token: &str,
        owner_id: &AccountId,
        amount: u128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.poa_factory_ft_deposit(self.id(), token, owner_id, amount, msg, memo)
            .await
    }

    async fn poa_factory_tokens(
        &self,
        poa_factory: &AccountId,
    ) -> anyhow::Result<HashMap<String, AccountId>> {
        self.view(poa_factory, "tokens")
            .await?
            .json()
            .map_err(Into::into)
    }
}

impl PoAFactoryExt for near_workspaces::Contract {
    async fn deploy_poa_factory(
        &self,
        name: &str,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
    ) -> anyhow::Result<Contract> {
        self.as_account()
            .deploy_poa_factory(name, super_admins, admins, grantees)
            .await
    }

    async fn poa_factory_deploy_token(
        &self,
        factory: &AccountId,
        token: &str,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> anyhow::Result<AccountId> {
        self.as_account()
            .poa_factory_deploy_token(factory, token, metadata)
            .await
    }

    async fn poa_deploy_token(
        &self,
        token: &str,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> anyhow::Result<AccountId> {
        self.as_account().poa_deploy_token(token, metadata).await
    }

    async fn poa_factory_ft_deposit(
        &self,
        factory: &AccountId,
        token: &str,
        owner_id: &AccountId,
        amount: u128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.as_account()
            .poa_factory_ft_deposit(factory, token, owner_id, amount, msg, memo)
            .await
    }

    async fn poa_ft_deposit(
        &self,
        token: &str,
        owner_id: &AccountId,
        amount: u128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.as_account()
            .poa_ft_deposit(token, owner_id, amount, msg, memo)
            .await
    }

    async fn poa_factory_tokens(
        &self,
        poa_factory: &AccountId,
    ) -> anyhow::Result<HashMap<String, AccountId>> {
        self.as_account().poa_factory_tokens(poa_factory).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::utils::{Sandbox, ft::FtExt};

    #[tokio::test]
    async fn test_deploy_mint() {
        let sandbox = Sandbox::new().await.unwrap();
        let root = sandbox.root_account();
        let user = sandbox.create_account("user1").await;

        let poa_factory = root
            .deploy_poa_factory(
                "poa-factory",
                [root.id().clone()],
                [
                    (Role::TokenDeployer, [root.id().clone()]),
                    (Role::TokenDepositer, [root.id().clone()]),
                ],
                [
                    (Role::TokenDeployer, [root.id().clone()]),
                    (Role::TokenDepositer, [root.id().clone()]),
                ],
            )
            .await
            .unwrap();

        user.poa_factory_deploy_token(poa_factory.id(), "ft1", None)
            .await
            .unwrap_err();

        root.poa_factory_deploy_token(poa_factory.id(), "ft1.abc", None)
            .await
            .unwrap_err();

        let ft1 = root
            .poa_factory_deploy_token(poa_factory.id(), "ft1", None)
            .await
            .unwrap();

        root.poa_factory_deploy_token(poa_factory.id(), "ft1", None)
            .await
            .unwrap_err();

        assert_eq!(
            sandbox.ft_token_balance_of(&ft1, user.id()).await.unwrap(),
            0
        );

        sandbox
            .ft_storage_deposit_many(&ft1, &[root.id(), user.id()])
            .await
            .unwrap();

        user.poa_factory_ft_deposit(poa_factory.id(), "ft1", user.id(), 1000, None, None)
            .await
            .unwrap_err();

        root.poa_factory_ft_deposit(poa_factory.id(), "ft1", user.id(), 1000, None, None)
            .await
            .unwrap();

        assert_eq!(
            sandbox.ft_token_balance_of(&ft1, user.id()).await.unwrap(),
            1000
        );
    }
}
