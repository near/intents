use defuse::core::payload::{DefusePayload, ExtractDefusePayload};
use defuse::core::{Nonce, intents::DefuseIntents, payload::multi::MultiPayload};
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
