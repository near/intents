use super::{DefusePayload, ExtractDefusePayload};
use defuse_erc191::SignedErc191Payload;
use near_sdk::{serde::de::DeserializeOwned, serde_json};

impl<T> ExtractDefusePayload<T> for SignedErc191Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        serde_json::from_str(&self.payload.0)
    }
}
