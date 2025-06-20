use defuse_sep53::{Sep53Payload, SignedSep53Payload};
use near_sdk::{
    serde::de::{DeserializeOwned, Error},
    serde_json,
};

use crate::payload::{DefusePayload, ExtractDefusePayload};

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
        let message_string = String::from_utf8(self.message).map_err(|_| {
            serde_json::Error::custom("Invalid SEP-53 message. Only UTF-8 strings are supported.")
        })?;
        let p: DefusePayload<T> = serde_json::from_str(&message_string)?;

        Ok(p)
    }
}
