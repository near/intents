use near_kit::{Final, InMemorySigner, KeyPair, Near};
use near_sdk::NearToken;

pub trait Account {
    async fn generate_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: impl Into<Option<NearToken>>,
    ) -> Near
    where
        Self: Sized;
}

impl Account for Near {
    async fn generate_subaccount(
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
            .transaction(account_id.clone())
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
            .expect("create subaccount failed");

        self.with_signer(
            InMemorySigner::from_secret_key(&account_id, kp.secret_key)
                .expect("key generation failed"),
        )
    }
}
