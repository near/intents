use near_kit::{Action, Final, FunctionCallAction, InMemorySigner, KeyPair, Near, NearToken};

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
    ) -> Self {
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

    async fn create_implicit(&self, balance: impl Into<Option<NearToken>>) -> Self {
        let kp = KeyPair::random();
        let account_id = defuse_core::PublicKey::Ed25519(
            *kp.public_key
                .as_ed25519_bytes()
                .expect("should return valid ed25519 pubkey"),
        )
        .to_implicit_account_id();

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

    async fn deploy_sub_contract(
        &self,
        name: impl AsRef<str>,
        balance: NearToken,
        code: impl Into<Vec<u8>>,
        init_call: impl Into<Option<FunctionCallAction>>,
    ) -> anyhow::Result<Self> {
        let kp = KeyPair::random();
        let account_id = self
            .account_id()
            .sub_account(name)
            .expect("failed to generate subaccount ID");

        let mut tx = self
            .transaction(&account_id)
            .create_account()
            .transfer(balance)
            .add_full_access_key(kp.public_key)
            .deploy(code.into());

        if let Some(init_call) = init_call.into() {
            tx = tx.add_action(Action::FunctionCall(init_call));
        }

        tx.wait_until(Final)
            .await?
            .result()
            .map_err(|e| anyhow::anyhow!("failed to deploy sub contract: {e:?}"))?;

        Ok(self.with_signer(InMemorySigner::from_secret_key(account_id, kp.secret_key).unwrap()))
    }
}
