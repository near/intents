use defuse_sr25519::SignedSr25519Payload;
use near_sdk::serde::de::DeserializeOwned;

use super::{DefusePayload, ExtractDefusePayload};

impl<T> ExtractDefusePayload<T> for SignedSr25519Payload
where
    T: DeserializeOwned,
{
    type Error = near_sdk::serde_json::Error;

    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        near_sdk::serde_json::from_str(&self.message)
    }
}
