use defuse_bip322::SignedBip322Payload;
use near_sdk::{serde::de::DeserializeOwned, serde_json};

use crate::payload::ExtractDefusePayload;

impl<T> ExtractDefusePayload<T> for SignedBip322Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn extract_defuse_payload(self) -> Result<super::DefusePayload<T>, Self::Error> {
        // Similar to ERC-191: parse the message field as JSON
        // The message field should contain a serialized DefusePayload<T>
        serde_json::from_str(&self.message)
    }
}
