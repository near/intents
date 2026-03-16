use crate::payload::{DefusePayload, ExtractDefusePayload};
use defuse_sep53::{Sep53Payload, SignedSep53Payload};
use near_sdk::{serde::de::DeserializeOwned, serde_json};

impl<T> ExtractDefusePayload<T> for SignedSep53Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        self.payload.extract_defuse_payload()
    }
}

impl<T> ExtractDefusePayload<T> for Sep53Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        serde_json::from_str(&self.payload)
    }
}
