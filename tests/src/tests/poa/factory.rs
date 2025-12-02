use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use defuse_poa_factory::contract::Role;
use defuse_sandbox::{
    Account, SigningAccount, extensions::account::AccountDeployerExt, tx::FnCallBuilder,
};
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::{AccountId, Gas, NearToken, json_types::U128};
use serde_json::json;

use crate::utils::read_wasm;

const POA_TOKEN_INIT_BALANCE: NearToken = NearToken::from_near(4);
static POA_FACTORY_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("res/defuse_poa_factory"));

pub trait PoAFactoryExt {
    async fn deploy_poa_factory(
        &self,
        name: &str,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
    ) -> anyhow::Result<Account>;

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

impl PoAFactoryExt for SigningAccount {
    async fn deploy_poa_factory(
        &self,
        name: &str,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
    ) -> anyhow::Result<Account> {
        let args = json!({
            "super_admins": super_admins.into_iter().collect::<HashSet<_>>(),
            "admins": admins
                .into_iter()
                .map(|(role, admins)| (role, admins.into_iter().collect::<HashSet<_>>()))
                .collect::<HashMap<_, _>>(),
            "grantees": grantees
                .into_iter()
                .map(|(role, grantees)| (role, grantees.into_iter().collect::<HashSet<_>>()))
                .collect::<HashMap<_, _>>(),
        });

        self.deploy_contract(
            name,
            POA_FACTORY_WASM.as_slice(),
            Some(FnCallBuilder::new("new").json_args(&args)),
        )
        .await
    }

    async fn poa_factory_deploy_token(
        &self,
        factory: &AccountId,
        token: &str,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> anyhow::Result<AccountId> {
        self.tx(factory.clone())
            .function_call(
                FnCallBuilder::new("deploy_token")
                    .json_args(&json!({
                        "token": token,
                        "metadata": metadata.into(),
                    }))
                    .with_deposit(NearToken::from_near(POA_TOKEN_INIT_BALANCE.as_near())),
            )
            .await?;

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
        self.tx(factory.clone())
            .function_call(
                FnCallBuilder::new("ft_deposit")
                    .json_args(&json!({
                            "token": token,
                            "owner_id": owner_id,
                            "amount": U128(amount),
                        "msg": msg,
                        "memo": memo,
                    }))
                    .with_deposit(NearToken::from_millinear(4)),
            )
            .await?;

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
        let account = Account::new(poa_factory.clone(), self.network_config().clone());
        account
            .call_view_function_json("tokens", ())
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use defuse_sandbox::{
        Sandbox,
        extensions::ft::{FtExt, FtViewExt},
    };
    use futures::try_join;
    use rstest::rstest;

    use super::*;

    #[tokio::test]
    #[rstest]
    async fn deploy_mint() {
        let sandbox = Sandbox::new().await;
        let root = sandbox.root();
        let user = sandbox
            .create_account("user1")
            .await
            .expect("Failed to create user");

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

        assert_eq!(root.ft_balance_of(&ft1, user.id()).await.unwrap(), 0);

        try_join!(
            root.ft_storage_deposit(&ft1, Some(root.id())),
            root.ft_storage_deposit(&ft1, Some(user.id()))
        )
        .unwrap();

        user.poa_factory_ft_deposit(poa_factory.id(), "ft1", user.id(), 1000, None, None)
            .await
            .unwrap_err();

        root.poa_factory_ft_deposit(poa_factory.id(), "ft1", user.id(), 1000, None, None)
            .await
            .unwrap();

        assert_eq!(root.ft_balance_of(&ft1, user.id()).await.unwrap(), 1000);
    }
}
