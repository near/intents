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
            // TON Connect [specification](https://docs.tonconsole.com/academy/sign-data#in-a-smart-contract-on-chain)
            // requires to check that "timestamp is recent". We don't have fixed TTL for
            // off-chain signatures but rather check if `deadline` is not expired.
            // So, it makes sense to at least check that the signature doesn't come from the future
            return Err(Error::custom("timestamp has not come yet"));
        }
        let TonConnectPayloadSchema::Text { text } = self.payload else {
            return Err(Error::custom("only text payload supported"));
        };
        serde_json::from_str(&text)
    }
}
