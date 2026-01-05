use defuse::core::payload::{DefusePayload, ExtractDefusePayload};
use defuse::core::{Deadline, ExpirableNonce, Salt, SaltedNonce, VersionedNonce};
use defuse::core::{Nonce, intents::DefuseIntents, payload::multi::MultiPayload};
use defuse_randomness::Rng;
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

pub fn create_random_salted_nonce(salt: Salt, deadline: Deadline, mut rng: impl Rng) -> Nonce {
    VersionedNonce::V1(SaltedNonce::new(
        salt,
        ExpirableNonce {
            deadline,
            nonce: rng.random::<[u8; 15]>(),
        },
    ))
    .into()
}
