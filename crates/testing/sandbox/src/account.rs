use defuse_core::crypto::ed25519::Ed25519PublicKey;
use near_kit::{
    NearToken, SigningNear,
    protocol::{Action, FunctionCallAction},
    signer::{InMemorySigner, SecretKey},
};

pub trait Account {
    async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> SigningNear;

    async fn create_implicit(&self, balance: impl Into<Option<NearToken>>) -> SigningNear;

    async fn deploy_sub_contract(
        &self,
        name: impl AsRef<str>,
        balance: NearToken,
        code: impl Into<Vec<u8>>,
        init_call: impl Into<Option<FunctionCallAction>>,
    ) -> anyhow::Result<SigningNear>;
}

impl Account for SigningNear {
    async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> Self {
        let secret_key = SecretKey::generate_ed25519();
        let account_id = self
            .account_id()
            .sub_account(name)
            .expect("Failed to generate subaccount ID");

        let mut tx = self
            .transaction(&account_id)
            .add_action(Action::create_account())
            .add_action(Action::add_full_access_key(secret_key.public_key()));

        if let Some(balance) = balance.into() {
            tx = tx.add_action(Action::transfer(balance));
        }

        tx.await
            .unwrap()
            .result()
            .expect("failed to create subaccount");

        self.with_signer(InMemorySigner::new(account_id, secret_key))
    }

    async fn create_implicit(&self, balance: impl Into<Option<NearToken>>) -> Self {
        let secret_key = SecretKey::generate_ed25519();
        let account_id = defuse_core::PublicKey::Ed25519(Ed25519PublicKey(
            *secret_key
                .public_key()
                .as_ed25519_bytes()
                .expect("should return valid ed25519 pubkey"),
        ))
        .to_implicit_account_id();

        if let Some(balance) = balance.into() {
            self.transaction(&account_id)
                .add_action(Action::transfer(balance))
                .await
                .unwrap()
                .result()
                .expect("implicit account funding failed");
        }

        self.with_signer(InMemorySigner::new(account_id, secret_key))
    }

    async fn deploy_sub_contract(
        &self,
        name: impl AsRef<str>,
        balance: NearToken,
        code: impl Into<Vec<u8>>,
        init_call: impl Into<Option<FunctionCallAction>>,
    ) -> anyhow::Result<Self> {
        let secret_key = SecretKey::generate_ed25519();
        let account_id = self
            .account_id()
            .sub_account(name)
            .expect("failed to generate subaccount ID");

        let mut tx = self
            .transaction(&account_id)
            .add_action(Action::create_account())
            .add_action(Action::transfer(balance))
            .add_action(Action::add_full_access_key(secret_key.public_key()))
            .add_action(Action::deploy_contract(code));

        if let Some(init_call) = init_call.into() {
            tx = tx.add_action(Action::FunctionCall(init_call));
        }

        tx.await?.result().map_err(|e| {
            anyhow::anyhow!("failed to deploy sub contract to '{account_id}': {e:?}")
        })?;

        Ok(self.with_signer(InMemorySigner::new(account_id, secret_key)))
    }
}
