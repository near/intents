use defuse_core::crypto::ed25519::Ed25519PublicKey;
use near_kit::{
    AccountId, Near, NearToken,
    protocol::{Action, FunctionCallAction},
    signer::{InMemorySigner, SecretKey},
    transaction::Final,
};

pub trait Account {
    fn signer_id(&self) -> &AccountId;

    async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> Near;

    async fn create_implicit(&self, balance: impl Into<Option<NearToken>>) -> Near;

    async fn deploy_sub_contract(
        &self,
        name: impl AsRef<str>,
        balance: NearToken,
        code: impl Into<Vec<u8>>,
        init_call: impl Into<Option<FunctionCallAction>>,
    ) -> anyhow::Result<Near>;
}

impl Account for Near {
    #[inline]
    fn signer_id(&self) -> &AccountId {
        self.account_id().expect("client has no signer")
    }

    async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> Self {
        let secret_key = SecretKey::generate_ed25519();
        let account_id = self
            .signer_id()
            .sub_account(name)
            .expect("Failed to generate subaccount ID");

        let mut tx = self
            .transaction(&account_id)
            .create_account()
            .add_full_access_key(secret_key.public_key());

        if let Some(balance) = balance.into() {
            tx = tx.transfer(balance);
        }

        tx.send()
            .wait_until::<Final>()
            .await
            .unwrap()
            .result()
            .expect("failed to create subaccount");

        self.with_signer(InMemorySigner::from_secret_key(account_id, secret_key).unwrap())
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
                .transfer(balance)
                .send()
                .wait_until::<Final>()
                .await
                .unwrap()
                .result()
                .expect("implicit account funding failed");
        }

        self.with_signer(InMemorySigner::from_secret_key(account_id, secret_key).unwrap())
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
            .signer_id()
            .sub_account(name)
            .expect("failed to generate subaccount ID");

        let mut tx = self
            .transaction(&account_id)
            .create_account()
            .transfer(balance)
            .add_full_access_key(secret_key.public_key())
            .deploy(code);

        if let Some(init_call) = init_call.into() {
            tx = tx.add_action(Action::FunctionCall(init_call));
        }

        tx.wait_until::<Final>().await?.result().map_err(|e| {
            anyhow::anyhow!("failed to deploy sub contract to '{account_id}': {e:?}")
        })?;

        Ok(self.with_signer(InMemorySigner::from_secret_key(account_id, secret_key).unwrap()))
    }
}
