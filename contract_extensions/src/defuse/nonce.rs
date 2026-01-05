use defuse::core::payload::{DefusePayload, ExtractDefusePayload};
use defuse::core::{Deadline, ExpirableNonce, Salt, SaltedNonce, VersionedNonce};
use defuse::core::{Nonce, intents::DefuseIntents, payload::multi::MultiPayload};
use defuse_randomness::{Rng, RngCore};
use defuse_sandbox::{Account, anyhow};
use defuse_test_utils::random::TestRng;
use near_sdk::serde_json;

use crate::defuse::state::SaltViewExt;

pub trait ExtractNonceExt {
    fn extract_nonce(&self) -> Result<Nonce, serde_json::Error>;
}

impl ExtractNonceExt for MultiPayload {
    #[inline]
    fn extract_nonce(&self) -> Result<Nonce, serde_json::Error> {
        let DefusePayload::<DefuseIntents> { nonce, .. } = self.clone().extract_defuse_payload()?;
        Ok(nonce)
    }
}

pub trait GenerateNonceExt {
    async fn generate_unique_nonce(
        &self,
        defuse_contract: &Account,
        deadline: Option<Deadline>,
    ) -> anyhow::Result<Nonce> {
        let deadline =
            deadline.unwrap_or_else(|| Deadline::timeout(std::time::Duration::from_secs(120)));

        let salt = defuse_contract
            .current_salt()
            .await
            .expect("should be able to fetch salt");

        let mut nonce_bytes = [0u8; 15];
        TestRng::from_entropy().fill_bytes(&mut nonce_bytes);

        let salted = SaltedNonce::new(salt, ExpirableNonce::new(deadline, nonce_bytes));
        Ok(VersionedNonce::V1(salted).into())
    }

    fn create_random_salted_nonce(salt: Salt, deadline: Deadline, mut rng: impl Rng) -> Nonce {
        VersionedNonce::V1(SaltedNonce::new(
            salt,
            ExpirableNonce {
                deadline,
                nonce: rng.random::<[u8; 15]>(),
            },
        ))
        .into()
    }
}

impl<T> GenerateNonceExt for T {}
