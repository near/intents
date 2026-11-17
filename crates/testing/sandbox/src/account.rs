use defuse_core::crypto::ed25519::Ed25519PublicKey;
use near_kit::{
    AccountIdRef, Error, NearToken, SigningNear,
    protocol::Action,
    rpc::{AccessKeyListView, AccountBalance, AccountView, Finality, RpcError},
    signer::{InMemorySigner, SecretKey},
    transaction::FunctionCall,
};

pub trait AccountExt {
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
        init_call: impl Into<Option<FunctionCall>>,
    ) -> anyhow::Result<SigningNear>;

    async fn balance(&self, account_id: impl AsRef<AccountIdRef>) -> Result<AccountBalance, Error>;

    async fn account(&self, account_id: impl AsRef<AccountIdRef>) -> Result<AccountView, RpcError>;

    async fn access_keys(
        &self,
        account_id: impl AsRef<AccountIdRef>,
    ) -> Result<AccessKeyListView, RpcError>;
}

impl AccountExt for SigningNear {
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
            .checked()
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
                .checked()
                .expect("implicit account funding failed");
        }

        self.with_signer(InMemorySigner::new(account_id, secret_key))
    }

    async fn deploy_sub_contract(
        &self,
        name: impl AsRef<str>,
        balance: NearToken,
        code: impl Into<Vec<u8>>,
        init_call: impl Into<Option<FunctionCall>>,
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
            tx = tx.add_action(init_call);
        }

        tx.await?.checked().map_err(|e| {
            anyhow::anyhow!("failed to deploy sub contract to '{account_id}': {e:?}")
        })?;

        Ok(self.with_signer(InMemorySigner::new(account_id, secret_key)))
    }

    async fn balance(&self, account_id: impl AsRef<AccountIdRef>) -> Result<AccountBalance, Error> {
        self.near().balance(account_id.as_ref()).await
    }

    async fn account(&self, account_id: impl AsRef<AccountIdRef>) -> Result<AccountView, RpcError> {
        self.near()
            .rpc()
            .view_account(account_id.as_ref(), Finality::Final)
            .await
    }

    async fn access_keys(
        &self,
        account_id: impl AsRef<AccountIdRef>,
    ) -> Result<AccessKeyListView, RpcError> {
        self.near()
            .rpc()
            .view_access_key_list(account_id.as_ref(), Finality::Final)
            .await
    }
}
