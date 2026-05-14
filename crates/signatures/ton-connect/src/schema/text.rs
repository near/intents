use std::fmt::Debug;

use crate::schema::{PayloadSchema, TonConnectPayloadContext};
use impl_tools::autoimpl;
use tlb_ton::StringError;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    serde(bound = ""),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[autoimpl(Deref using self.text)]
pub struct TextPayload {
    pub text: String,
}

impl PayloadSchema for TextPayload {
    fn hash_with_context<D: defuse_digest::Digest<OutputSize = defuse_digest::U32>>(
        &self,
        context: TonConnectPayloadContext,
    ) -> Result<defuse_crypto::CryptoHash, StringError> {
        context.create_payload_hash::<D>(b"txt", self.as_bytes())
    }
}
