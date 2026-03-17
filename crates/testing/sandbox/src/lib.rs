use anyhow::{Result, anyhow};
use impl_tools::autoimpl;
use near_kit::{
    AccountId, Action, FunctionCallAction, InMemorySigner, KeyPair, Near, NearToken,
    sandbox::SandboxConfig,
};
use rstest::fixture;
use std::sync::atomic::{AtomicUsize, Ordering};
pub mod extensions;

pub use near_kit;

pub const DEFAULT_DEPOSIT: NearToken = NearToken::from_near(100);

#[fixture]
pub async fn sandbox(#[default(NearToken::from_near(100_000))] amount: NearToken) -> Sandbox {
    static SUB_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let near = SandboxConfig::shared().await.client();
    let child_id = near
        .sub_account(SUB_COUNTER.fetch_add(1, Ordering::SeqCst).to_string())
        .unwrap();
    let key_pair = KeyPair::random();

    near.transaction(&child_id)
        .create_account()
        .transfer(amount)
        .add_full_access_key(key_pair.public_key)
        .send()
        .await
        .expect("Failed to create account");

    Sandbox {
        root: near.with_signer(InMemorySigner::from_secret_key(
            child_id,
            key_pair.secret_key,
        )),
    }
}

#[autoimpl(Deref using self.root)]
pub struct Sandbox {
    pub root: Near,
}

impl Sandbox {
    pub async fn generate_sub_account(
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

    pub async fn deploy_sub_contract(
        &self,
        name: impl AsRef<str>,
        balance: NearToken,
        code: impl Into<Vec<u8>>,
        init_call: impl Into<Option<FunctionCallAction>>,
    ) -> anyhow::Result<Near> {
        let key_pair = KeyPair::random();
        let subaccount = self.sub_account(name)?;

        let mut tx = self
            .transaction(&subaccount)
            .create_account()
            .transfer(balance)
            .add_full_access_key(key_pair.public_key)
            .deploy(code);

        if let Some(init_call) = init_call.into() {
            tx = tx.add_action(Action::FunctionCall(init_call));
        }
        tx.await?;

        Ok(self.with_signer(InMemorySigner::from_secret_key(
            subaccount,
            key_pair.secret_key,
        )))
    }
}

// TODO: this is hacky
pub trait ToNearKit {
    fn to_kit(&self) -> near_kit::AccountId;
}

pub trait ToNearSdk {
    fn to_sdk(&self) -> near_sdk::AccountId;
}

impl ToNearKit for near_sdk::AccountId {
    fn to_kit(&self) -> near_kit::AccountId {
        self.as_str().parse().unwrap()
    }
}

impl ToNearSdk for near_kit::AccountId {
    fn to_sdk(&self) -> near_sdk::AccountId {
        self.as_str().parse().unwrap()
    }
}

// TODO: total shit
pub trait IntoAccountId<T>: Sized {
    fn into_account_id(self) -> T;
}

impl IntoAccountId<near_sdk::AccountId> for &Near {
    fn into_account_id(self) -> near_sdk::AccountId {
        self.account_id().unwrap().as_str().parse().unwrap()
    }
}

pub trait SubAcount {
    fn sub_account(&self, name: impl AsRef<str>) -> Result<AccountId>;
}

impl SubAcount for AccountId {
    fn sub_account(&self, name: impl AsRef<str>) -> Result<AccountId> {
        format!("{}.{}", name.as_ref(), self)
            .parse()
            .map_err(Into::into)
    }
}

impl SubAcount for Near {
    fn sub_account(&self, name: impl AsRef<str>) -> Result<AccountId> {
        let parent_id = self.account_id().ok_or(anyhow!("Account should exist"))?;

        parent_id.sub_account(name)
    }
}
