use anyhow::{Result, anyhow};
use defuse::contract::config::DefuseConfig;
use defuse_poa_factory::contract::Role;
use impl_tools::autoimpl;
use near_kit::{AccountId, InMemorySigner, KeyPair, Near, NearToken, sandbox::SandboxConfig};
use rstest::fixture;
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicUsize, Ordering},
};

pub mod extensions;

pub use near_kit;

#[fixture]
pub async fn sandbox(#[default(NearToken::from_near(100_000))] amount: NearToken) -> Sandbox {
    static SUB_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let near = SandboxConfig::shared().await.client();
    let parent_id = near
        .account_id()
        .expect("Parent account id should be present");

    let key_pair = KeyPair::random();
    let child_id = format!(
        "{}.{}",
        SUB_COUNTER.fetch_add(1, Ordering::SeqCst),
        parent_id
    );

    near.transaction(&child_id)
        .create_account()
        .transfer(amount)
        .add_full_access_key(key_pair.public_key)
        .send()
        .await
        .expect("Failed to create account");

    Sandbox {
        root: near.with_signer(InMemorySigner::from_secret_key(
            child_id.parse().unwrap(),
            key_pair.secret_key,
        )),
    }
}

#[autoimpl(Deref using self.root)]
pub struct Sandbox {
    root: Near,
}

impl Sandbox {
    pub fn sub_account(&self, name: impl AsRef<str>) -> anyhow::Result<AccountId> {
        let parent_id = self.account_id().ok_or(anyhow!("Account should exist"))?;

        let child_id = format!("{}.{}", name.as_ref(), parent_id);
        Ok(child_id.parse()?)
    }

    pub async fn generate_subaccount(
        &self,
        name: impl AsRef<str>,
        amount: NearToken,
    ) -> Result<Near> {
        let key_pair = KeyPair::random();
        let child_id = self.sub_account(name)?;

        self.transaction(&child_id)
            .create_account()
            .transfer(amount)
            .add_full_access_key(key_pair.public_key)
            .send()
            .await?;

        Ok(self.with_signer(InMemorySigner::from_secret_key(
            child_id,
            key_pair.secret_key,
        )))
    }

    pub async fn deploy_poa_factory(
        &self,
        name: impl AsRef<str>,
        super_admins: impl IntoIterator<Item = AccountId>,
        admins: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        grantees: impl IntoIterator<Item = (Role, impl IntoIterator<Item = AccountId>)>,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<Near> {
        let key_pair = KeyPair::random();
        let subaccount = self.sub_account(name)?;

        self.transaction(&subaccount)
            .create_account()
            .transfer(NearToken::from_near(100))
            .add_full_access_key(key_pair.public_key)
            .deploy(wasm.into())
            .call("new")
            .args(json!({
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
            .await?;

        Ok(self.with_signer(InMemorySigner::from_secret_key(
            subaccount,
            key_pair.secret_key,
        )))
    }

    pub async fn deploy_wrap_near(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<Near> {
        let key_pair = KeyPair::random();
        let subaccount = self.sub_account(name)?;

        self.transaction(&subaccount)
            .create_account()
            .transfer(NearToken::from_near(100))
            .add_full_access_key(key_pair.public_key)
            .deploy(wasm.into())
            .call("new")
            .await?;

        Ok(self.with_signer(InMemorySigner::from_secret_key(
            subaccount,
            key_pair.secret_key,
        )))
    }

    pub async fn deploy_defuse(
        &self,
        name: impl AsRef<str>,
        config: DefuseConfig,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<Near> {
        let key_pair = KeyPair::random();
        let subaccount = self.sub_account(name)?;

        self.transaction(&subaccount)
            .create_account()
            .transfer(NearToken::from_near(100))
            .add_full_access_key(key_pair.public_key)
            .deploy(wasm.into())
            .call("new")
            .args(json!({
                "config": config,
            }))
            .await?;

        Ok(self.with_signer(InMemorySigner::from_secret_key(
            subaccount,
            key_pair.secret_key,
        )))
    }

    // pub async fn deploy_sub_contract(
    //     &self,
    //     name: impl AsRef<str>,
    //     balance: NearToken,
    //     code: impl Into<Vec<u8>>,
    //     // init_call: impl Into<Option<FnCallBuilder>>,
    // ) -> anyhow::Result<Near> {
    //     let key_pair = KeyPair::random();
    //     let subaccount = self.sub_account(name)?;

    //     // self.transaction(&subaccount)
    //     //     .create_account()
    //     //     .transfer(balance)
    //     //     .add_full_access_key(key_pair.public_key)
    //     //     .deploy(code)

    //     // if let Some(init_call) = init_call.into() {
    //     //     tx = tx.call(method)
    //     // }
    //     // tx.await?;

    //     // Ok(Self::new(
    //     //     subaccount,
    //     //     Signer::from_secret_key(secret_key).unwrap(),
    //     // ))
    // }
}
