use std::fmt::Debug;

use crate::schema::{PayloadSchema, TonConnectPayloadContext};
use defuse_serde_utils::base64::Base64;
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
    cfg_attr(
        feature = "abi",
        schemars(bound = ""),
        schemars(rename = "BinaryPayload"),
        derive(::schemars::JsonSchema)
    )
)]
#[autoimpl(Deref using self.bytes)]
pub struct BinaryPayload<D = ()> {
    #[cfg_attr(feature = "serde", serde_as(as = "Base64"))]
    pub bytes: Vec<u8>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "abi", schemars(skip))]
    pub _phantom: std::marker::PhantomData<D>,
}

impl<D: digest::Digest<OutputSize = digest::consts::U32>> PayloadSchema for BinaryPayload<D> {
    fn hash_with_context(
        &self,
        context: TonConnectPayloadContext,
    ) -> Result<defuse_crypto::CryptoHash, StringError> {
        context.create_payload_hash::<D>(b"bin", self.as_slice())
    }
}
