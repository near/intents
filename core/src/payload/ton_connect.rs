use defuse_ton_connect::{SignedTonConnectPayload, TonConnectPayload, TonConnectPayloadSchema};
use near_sdk::{
    serde::de::{DeserializeOwned, Error},
    serde_json,
};

use super::{DefusePayload, ExtractDefusePayload};

impl<T> ExtractDefusePayload<T> for SignedTonConnectPayload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        self.payload.extract_defuse_payload()
    }
}

impl<T> ExtractDefusePayload<T> for TonConnectPayload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        if self.timestamp > defuse_near_utils::time::now() {
            return Err(Error::custom("timestamp has not come yet"));
        }
        let TonConnectPayloadSchema::Text { text } = self.payload else {
            return Err(Error::custom("only text payload supported"));
        };
        serde_json::from_str(&text)
    }
}
