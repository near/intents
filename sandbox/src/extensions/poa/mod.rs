#![allow(async_fn_in_trait)]

pub use defuse_poa_factory as contract;

use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::{AccountId, AccountIdRef, NearToken, json_types::U128, serde_json::json};
use std::collections::{HashMap, HashSet};

use defuse_poa_factory::contract::{POA_TOKEN_INIT_BALANCE, Role};

use crate::{Account, SigningAccount, anyhow, tx::FnCallBuilder};

pub trait PoAFactoryExt {
    async fn poa_factory_deploy_token(
        &self,
        factory: impl Into<AccountId>,
        token: impl AsRef<str>,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> anyhow::Result<Account>;

    async fn poa_factory_ft_deposit(
        &self,
        factory: impl Into<AccountId>,
        token: impl AsRef<str>,
        owner_id: impl AsRef<AccountIdRef>,
        amount: u128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> anyhow::Result<()>;
}

impl PoAFactoryExt for SigningAccount {
    async fn poa_factory_deploy_token(
        &self,
        factory: impl Into<AccountId>,
        token: impl AsRef<str>,
        metadata: impl Into<Option<FungibleTokenMetadata>>,
    ) -> anyhow::Result<Account> {
        let factory = factory.into();
        let token = token.as_ref();

        let account = Account::new(factory.sub_account(token)?, self.network_config().clone());

        self.tx(factory)
            .function_call(
                FnCallBuilder::new("deploy_token")
                    .json_args(json!({
                        "token": token,
                        "metadata": metadata.into(),
                    }))
                    .with_deposit(NearToken::from_near(POA_TOKEN_INIT_BALANCE.as_near())),
            )
            .await?;

        Ok(account)
    }

    async fn poa_factory_ft_deposit(
        &self,
        factory: impl Into<AccountId>,
        token: impl AsRef<str>,
        owner_id: impl AsRef<AccountIdRef>,
        amount: u128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.tx(factory)
            .function_call(
                FnCallBuilder::new("ft_deposit")
                    .json_args(json!({
                        "token": token.as_ref(),
                        "owner_id": owner_id.as_ref(),
                        "amount": U128(amount),
                        "msg": msg,
                        "memo": memo,
                    }))
                    .with_deposit(NearToken::from_millinear(4)),
            )
            .await?;

        Ok(())
    }
}

pub trait PoAFactoryViewExt {
    async fn poa_tokens(
        &self,
        poa_factory: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<HashMap<String, AccountId>>;
}

pub trait PoAFactoryDeployerExt {
    async fn deploy_poa_factory(
        &self,
        name: impl AsRef<str>,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<SigningAccount>;
}

impl PoAFactoryDeployerExt for SigningAccount {
    async fn deploy_poa_factory(
        &self,
        name: impl AsRef<str>,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<Self> {
        self.deploy_sub_contract(
            name,
            NearToken::from_near(100),
            wasm.into(),
            Some(FnCallBuilder::new("new").json_args(json!({
                "super_admins": super_admins.into_iter().collect::<HashSet<_>>(),
                "admins": admins
                    .into_iter()
                    .map(|(role, admins)| (role, admins.into_iter().collect::<HashSet<_>>()))
                    .collect::<HashMap<_, _>>(),
                "grantees": grantees
                    .into_iter()
                    .map(|(role, grantees)| (role, grantees.into_iter().collect::<HashSet<_>>()))
                    .collect::<HashMap<_, _>>(),
            }))),
        )
        .await
    }
}
