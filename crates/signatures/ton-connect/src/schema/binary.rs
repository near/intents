use std::fmt::Debug;

use crate::schema::{PayloadSchema, TonConnectPayloadContext};
use impl_tools::autoimpl;
use tlb_ton::StringError;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[autoimpl(Deref using self.bytes)]
pub struct BinaryPayload {
    #[cfg_attr(feature = "serde", serde_as(as = "defuse_serde_utils::base64::Base64"))]
    pub bytes: Vec<u8>,
}

impl PayloadSchema for BinaryPayload {
    fn hash_with_context(
        &self,
        context: TonConnectPayloadContext,
    ) -> Result<defuse_crypto::CryptoHash, StringError> {
        context.create_payload_hash(b"bin", self.as_slice())
    }
}
