use defuse::core::{intents::DefuseIntents, payload::multi::MultiPayload, Nonce};
use defuse::core::payload::{DefusePayload, ExtractDefusePayload};

pub trait ExtractNonceExt {
    fn extract_nonce(&self) -> Result<Nonce,serde_json::Error>;
}

impl ExtractNonceExt for MultiPayload
{
    #[inline]
    fn extract_nonce(&self) -> Result<Nonce,serde_json::Error>  {
        let DefusePayload::<DefuseIntents> {
            nonce, ..
        } = self.clone().extract_defuse_payload()?;
        Ok(nonce)
    }
}

