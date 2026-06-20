use std::fmt::Debug;

use impl_tools::autoimpl;
use tlb_ton::StringError;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[autoimpl(Deref using self.text)]
pub struct TextPayload {
    pub text: String,
}

impl crate::schema::PayloadSchema for TextPayload {
    fn hash_with_context(
        &self,
        context: crate::schema::TonConnectPayloadContext,
    ) -> Result<defuse_crypto::CryptoHash, StringError> {
        context.create_payload_hash(b"txt", self.as_bytes())
    }
}
