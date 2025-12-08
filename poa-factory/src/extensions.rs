#[allow(async_fn_in_trait)]
use defuse_sandbox::{
    Account, SigningAccount, anyhow, extensions::account::AccountDeployerExt, tx::FnCallBuilder,
};
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::{AccountId, NearToken, json_types::U128, serde_json::json};
use std::collections::{HashMap, HashSet};

use crate::contract::{POA_TOKEN_INIT_BALANCE, Role};

// TODO: make it prettier
const POA_FACTORY_WASM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../releases/defuse_poa_factory.wasm"
));

#[allow(async_fn_in_trait)]
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

    async fn poa_factory_ft_deposit(
        &self,
        factory: &AccountId,
        token: &str,
        owner_id: &AccountId,
        amount: u128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> anyhow::Result<()>;
}

#[allow(async_fn_in_trait)]
pub trait PoAFactoryViewExt {
    async fn poa_tokens(
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
            POA_FACTORY_WASM,
            NearToken::from_near(100),
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
}

impl PoAFactoryViewExt for SigningAccount {
    async fn poa_tokens(
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
