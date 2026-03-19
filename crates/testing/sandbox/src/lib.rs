use anyhow::{Result, anyhow};
use impl_tools::autoimpl;
use near_kit::{
    AccountId, Action, FunctionCallAction, InMemorySigner, KeyPair, Near, NearToken,
    sandbox::SandboxConfig,
};
use near_sdk::Gas;
use rstest::fixture;
use std::sync::atomic::{AtomicUsize, Ordering};
pub mod extensions;

pub use near_kit;

pub const DEFAULT_GAS: Gas = Gas::from_tgas(300);

pub const MAX_NONCE_RETRIES: u32 = 1000;

#[fixture]
pub async fn sandbox(#[default(NearToken::from_near(10_000))] amount: NearToken) -> Sandbox {
    static SUB_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let cfg = SandboxConfig::shared().await;
    let near = cfg.client();
    // Set a large fixed balance so concurrent test fixtures don't race with each other.
    // set_balance overwrites (not adds), so using amount+small would be exhausted by
    // parallel tests. 100_000_000 NEAR gives plenty of headroom.
    cfg.set_balance(
        near.account_id().unwrap(),
        NearToken::from_near(100_000_000),
    )
    .await
    .expect("Failed to boost sandbox root balance");

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

    let mut root = near.with_signer(InMemorySigner::from_secret_key(
        child_id,
        key_pair.secret_key,
    ));

    root.set_max_nonce_retries(MAX_NONCE_RETRIES);

    Sandbox { root }
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
            .wait_until(near_kit::TxExecutionStatus::Final)
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
    ) -> anyhow::Result<AccountId> {
        let subaccount = self.sub_account(name)?;

        let mut tx = self
            .transaction(&subaccount)
            .create_account()
            .transfer(balance)
            .add_full_access_key(self.public_key().expect("Public key should be present"))
            .deploy(code);

        if let Some(init_call) = init_call.into() {
            tx = tx.add_action(Action::FunctionCall(init_call));
        }
        tx.await?;

        Ok(subaccount)
    }

    pub async fn deploy_global_contract_by_hash(
        &self,
        code: impl Into<Vec<u8>>,
    ) -> anyhow::Result<()> {
        self.transaction(self.account_id().unwrap())
            .publish_contract(code, true)
            .send()
            .await?;

        Ok(())
    }

    // pub async fn deploy_global_contract_by_account_id(
    //     &self,
    //     account_id: impl Into<AccountId>,
    //     code: impl Into<Vec<u8>>,
    //     mode: GlobalContractDeployMode,
    // ) -> anyhow::Result<()> {
    //     self.transaction(account_id.into())
    //         .publish_contract(code, by_hash)
    //         .send()
    //         .await?;

    //     Ok(())
    // }
}

// TODO: total shit
pub trait IntoAccountId<T>: Sized {
    fn into_account_id(self) -> T;
}

impl IntoAccountId<near_sdk::AccountId> for &Near {
    fn into_account_id(self) -> near_sdk::AccountId {
        self.account_id().unwrap().clone()
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
