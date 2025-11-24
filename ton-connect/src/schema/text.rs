use core::str;
use std::fmt::Debug;

use impl_tools::autoimpl;
use near_sdk::near;
use serde_with::serde_as;
use tlb_ton::StringError;

use crate::schema::{PayloadSchema, TonConnectPayloadContext};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(arbitrary::Arbitrary))]
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[autoimpl(Deref using self.text)]
pub struct TextPayload {
    pub text: String,
}

impl PayloadSchema for TextPayload {
    fn hash_with_context(
        &self,
        context: TonConnectPayloadContext,
    ) -> Result<near_sdk::CryptoHash, StringError> {
        context.create_payload_hash(b"txt", self.as_bytes())
    }
}
