use core::fmt::Display;

#[cfg(feature = "serde")]
use defuse_crypto::serde::AsCurve;
#[cfg(feature = "near-contract")]
use defuse_crypto::{CryptoHash, Curve, Payload, SignedPayload};
use defuse_crypto::{CurveTypes, Ed25519};
use defuse_nep461::{OffchainMessage, SignedMessageNep};
#[cfg(feature = "serde")]
use defuse_serde_utils::base64::Base64;
use impl_tools::autoimpl;
#[cfg(feature = "near-contract")]
use near_sdk::env;

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

    #[cfg_attr(feature = "serde", serde_as(as = "Base64"))]
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

    #[cfg(feature = "borsh")]
    #[inline]
    pub fn prehash(&self) -> Vec<u8> {
        borsh::to_vec(&(Self::OFFCHAIN_PREFIX_TAG, self)).expect("infallible")
    }
}

#[cfg(feature = "near-contract")]
impl Payload for Nep413Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        env::sha256_array(self.prehash())
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

    #[cfg_attr(feature = "serde", serde_as(as = "AsCurve<Ed25519>"))]
    pub public_key: <Ed25519 as CurveTypes>::PublicKey,
    #[cfg_attr(feature = "serde", serde_as(as = "AsCurve<Ed25519>"))]
    pub signature: <Ed25519 as CurveTypes>::Signature,
}

#[cfg(feature = "near-contract")]
impl Payload for SignedNep413Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        self.payload.hash()
    }
}

#[cfg(feature = "near-contract")]
impl SignedPayload for SignedNep413Payload {
    type PublicKey = <Ed25519 as CurveTypes>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Ed25519::verify(&self.signature, &self.hash(), &self.public_key)
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
