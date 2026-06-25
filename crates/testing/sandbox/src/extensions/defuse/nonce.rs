use crate::extensions::defuse::state::SaltViewExt;
use crate::{Account, anyhow};
use defuse::core::payload::{DefusePayload, ExtractDefusePayload};
use defuse::core::{ExpirableNonce, Salt, SaltedNonce, Timestamp, VersionedNonce};
use defuse::core::{Nonce, intents::DefuseIntents, payload::multi::MultiPayload};
use defuse_test_utils::random::{Rng, RngExt, TestRng};
use near_sdk::serde_json;

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

pub async fn generate_unique_nonce(
    defuse_contract: &Account,
    deadline: Option<Timestamp>,
) -> anyhow::Result<Nonce> {
    let deadline = deadline.unwrap_or_else(|| Timestamp::now() + std::time::Duration::from_mins(2));

    let salt = defuse_contract.current_salt().await?;

    Ok(create_random_salted_nonce(
        salt,
        deadline,
        TestRng::from_entropy(),
    ))
}

pub fn create_random_salted_nonce(salt: Salt, deadline: Timestamp, mut rng: impl Rng) -> Nonce {
    VersionedNonce::V1(SaltedNonce::new(
        salt,
        ExpirableNonce {
            deadline,
            nonce: rng.random::<[u8; 15]>(),
        },
    ))
    .into()
}
