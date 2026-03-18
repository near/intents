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
// pub const DEFAULT_DEPOSIT: NearToken = NearToken::from_near(100);

pub const MAX_NONCE_RETRIES: u32 = 1000;

// TODO: why sandbox has only 10000.00 NEAR?
#[fixture]
pub async fn sandbox(#[default(NearToken::from_near(1_000))] amount: NearToken) -> Sandbox {
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

    let mut root = near.with_signer(InMemorySigner::from_secret_key(
        child_id,
        key_pair.secret_key,
    ));

    root.set_max_nonce_retries(MAX_NONCE_RETRIES);

    Sandbox { root }
}

// pub async fn generate_rotating_sub_account(
//     parent: &Near,
//     name: impl AsRef<str>,
//     amount: NearToken,
//     signer_count: usize,
// ) -> Result<Near> {
//     let child_id = parent.sub_account(name)?;
//     let keys = (0..signer_count)
//         .map(|_| KeyPair::random())
//         .collect::<Vec<_>>();

//     let signer = RotatingSigner::new(
//         &child_id,
//         keys.iter().map(|key| key.secret_key.clone()).collect(),
//     )?;

//     keys.iter()
//         .fold(
//             parent
//                 .transaction(child_id)
//                 .create_account()
//                 .transfer(amount),
//             |tx, key| tx.add_full_access_key(key.public_key.clone()),
//         )
//         .send()
//         .await?;

//     Ok(parent.with_signer(signer))
// }

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
