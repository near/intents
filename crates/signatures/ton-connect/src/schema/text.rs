use std::fmt::Debug;

#[cfg(feature = "near-contract")]
use crate::schema::{PayloadSchema, TonConnectPayloadContext};
use impl_tools::autoimpl;
#[cfg(feature = "near-contract")]
use tlb_ton::StringError;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[autoimpl(Deref using self.text)]
pub struct TextPayload {
    pub text: String,
}

#[cfg(feature = "near-contract")]
impl PayloadSchema for TextPayload {
    fn hash_with_context(
        &self,
        context: TonConnectPayloadContext,
    ) -> Result<defuse_crypto::CryptoHash, StringError> {
        context.create_payload_hash(b"txt", self.as_bytes())
    }
}
