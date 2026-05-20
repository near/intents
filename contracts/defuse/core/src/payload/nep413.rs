use defuse_nep413::{Nep413Payload, SignedNep413Payload};
use impl_tools::autoimpl;
use near_sdk::{
    AccountId, near,
    serde::de::{self, DeserializeOwned},
    serde_json,
};

use crate::Deadline;

use super::{DefusePayload, ExtractDefusePayload};

#[near(serializers = [json])]
#[autoimpl(Deref using self.message)]
#[autoimpl(DerefMut using self.message)]
#[derive(Debug, Clone)]
pub struct Nep413DefuseMessage<T> {
    pub signer_id: AccountId,

    pub deadline: Deadline,

    #[serde(flatten)]
    pub message: T,
}

impl<T> ExtractDefusePayload<T> for Nep413Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        let Nep413DefuseMessage {
            signer_id,
            deadline,
            message,
        } = serde_json::from_str(&self.message)?;

        Ok(DefusePayload {
            signer_id,
            verifying_contract: self.recipient.parse().map_err(|_| {
                de::Error::invalid_value(de::Unexpected::Str(&self.recipient), &"AccountId")
            })?,
            deadline,
            nonce: self.nonce,
            message,
        })
    }
}

impl<T> ExtractDefusePayload<T> for SignedNep413Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        self.payload.extract_defuse_payload()
    }
}
