use core::fmt::Display;

use defuse_crypto::{Curve, Ed25519};
use defuse_nep461::SignedMessageNep;
use impl_tools::autoimpl;

/// See [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md)
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    serde(rename_all = "camelCase"),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone)]
pub struct Nep413Payload {
    pub message: String,

    #[cfg_attr(feature = "serde", serde_as(as = "defuse_serde_utils::base64::Base64"))]
    pub nonce: [u8; 32],

    pub recipient: String,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub callback_url: Option<String>,
}

impl SignedMessageNep for Nep413Payload {
    const NEP_NUMBER: u32 = 413;
}

impl Nep413Payload {
    #[inline]
    pub fn new(message: String) -> Self {
        Self {
            message,
            nonce: Default::default(),
            recipient: String::new(),
            callback_url: None,
        }
    }

    #[must_use]
    #[inline]
    pub const fn with_nonce(mut self, nonce: [u8; 32]) -> Self {
        self.nonce = nonce;
        self
    }

    #[must_use]
    #[inline]
    pub fn with_recipient<S>(mut self, recipient: S) -> Self
    where
        S: Display,
    {
        self.recipient = recipient.to_string();
        self
    }

    #[must_use]
    #[inline]
    pub fn with_callback_url(mut self, callback_url: String) -> Self {
        self.callback_url = Some(callback_url);
        self
    }

    #[cfg(feature = "prehash")]
    #[inline]
    pub fn prehash(&self) -> Vec<u8> {
        use defuse_nep461::OffchainMessage;
        borsh::to_vec(&(Self::OFFCHAIN_PREFIX_TAG, self)).expect("infallible")
    }
}

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[autoimpl(Deref using self.payload)]
#[derive(Debug, Clone)]
pub struct SignedNep413Payload {
    pub payload: Nep413Payload,

    #[cfg_attr(
        feature = "serde",
        serde_as(as = "defuse_crypto::serde::AsCurve<Ed25519>")
    )]
    pub public_key: <Ed25519 as Curve>::PublicKey,
    #[cfg_attr(
        feature = "serde",
        serde_as(as = "defuse_crypto::serde::AsCurve<Ed25519>")
    )]
    pub signature: <Ed25519 as Curve>::Signature,
}

#[cfg(all(test, feature = "serde", feature = "abi"))]
mod schema_tests {
    use super::*;
    use schemars::schema_for;

    fn prop(schema_val: &serde_json::Value, field: &str) -> serde_json::Value {
        schema_val["properties"][field].clone()
    }

    #[test]
    fn nonce_schema_is_string() {
        let schema = serde_json::to_value(schema_for!(Nep413Payload)).unwrap();
        assert_eq!(prop(&schema, "nonce")["type"], "string");
    }

    #[test]
    fn public_key_schema_is_string() {
        let schema = serde_json::to_value(schema_for!(SignedNep413Payload)).unwrap();
        assert_eq!(prop(&schema, "public_key")["type"], "string");
    }

    #[test]
    fn signature_schema_is_string() {
        let schema = serde_json::to_value(schema_for!(SignedNep413Payload)).unwrap();
        assert_eq!(prop(&schema, "signature")["type"], "string");
    }
}

#[cfg(feature = "near-api")]
const _: () = {
    impl From<Nep413Payload> for near_api::signer::NEP413Payload {
        fn from(payload: Nep413Payload) -> Self {
            Self {
                message: payload.message,
                nonce: payload.nonce,
                recipient: payload.recipient,
                callback_url: payload.callback_url,
            }
        }
    }
};
