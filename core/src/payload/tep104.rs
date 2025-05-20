use defuse_tep104::{SignedTep104Payload, Tep104Payload, Tep104SchemaPayload};
use near_sdk::{serde::de::DeserializeOwned, serde_json};

use super::{DefusePayload, ExtractDefusePayload};

impl<T> ExtractDefusePayload<T> for Tep104Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        // TODO: check timestamp < now
        match self.payload {
            Tep104SchemaPayload::Plaintext { payload } => serde_json::from_str(&payload),
        }
    }
}

impl<T> ExtractDefusePayload<T> for SignedTep104Payload
where
    T: DeserializeOwned,
{
    type Error = <Tep104Payload as ExtractDefusePayload<T>>::Error;

    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        self.payload.extract_defuse_payload()
    }
}
