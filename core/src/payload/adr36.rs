use crate::payload::{DefusePayload, ExtractDefusePayload};
use defuse_adr36::{Adr36Payload, SignedAdr36Payload};
use near_sdk::{serde::de::DeserializeOwned, serde_json};

impl<T> ExtractDefusePayload<T> for SignedAdr36Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        self.payload.extract_defuse_payload()
    }
}

impl<T> ExtractDefusePayload<T> for Adr36Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        serde_json::from_str(&self.message)
    }
}
