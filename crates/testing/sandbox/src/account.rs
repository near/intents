use near_account_id::AccountId;
use near_kit::{
    Action, Final, FunctionCallAction, InMemorySigner, KeyPair, Near, NearToken, PublicKey,
};

pub trait Account {
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
    async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> Near {
        let kp = KeyPair::random();
        let account_id = self
            .account_id()
            .sub_account(name)
            .expect("Failed to generate subaccount ID");

        let mut tx = self
            .transaction(&account_id)
            .create_account()
            .add_full_access_key(kp.public_key);

        if let Some(balance) = balance.into() {
            tx = tx.transfer(balance);
        }

        tx.send()
            .wait_until(Final)
            .await
            .unwrap()
            .result()
            .expect("failed to create subaccount");

        self.with_signer(InMemorySigner::from_secret_key(account_id, kp.secret_key).unwrap())
    }

    async fn create_implicit(&self, balance: impl Into<Option<NearToken>>) -> Near {
        let kp = KeyPair::random();
        let account_id = generate_implicit_account_id(&kp.public_key);

        if let Some(balance) = balance.into() {
            self.transaction(&account_id)
                .transfer(balance)
                .send()
                .wait_until(Final)
                .await
                .unwrap()
                .result()
                .expect("implicit account funding failed");
        }

        self.with_signer(InMemorySigner::from_secret_key(account_id, kp.secret_key).unwrap())
    }

    // TODO: create + deploy n 1 tx
    async fn deploy_sub_contract(
        &self,
        name: impl AsRef<str>,
        balance: NearToken,
        code: impl Into<Vec<u8>>,
        init_call: impl Into<Option<FunctionCallAction>>,
    ) -> anyhow::Result<Near> {
        let account = self.create_subaccount(name, balance).await;

        let mut tx = account.deploy(code);

        if let Some(init_call) = init_call.into() {
            tx = tx.add_action(Action::FunctionCall(init_call));
        }

        tx.wait_until(Final)
            .await?
            .result()
            .map_err(|e| anyhow::anyhow!("Failed to deploy sub contract: {:?}", e))?;

        Ok(account)
    }
}

pub fn generate_implicit_account_id(public_key: &PublicKey) -> AccountId {
    defuse_core::PublicKey::Ed25519(
        *public_key
            .as_ed25519_bytes()
            .expect("should return valid ed25519 pubkey"),
    )
    .to_implicit_account_id()
}
