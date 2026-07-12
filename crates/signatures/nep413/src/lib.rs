//! [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md)
//! Offchain Signing Standard

use core::fmt::Display;

use borsh::{BorshDeserialize, BorshSerialize};
use defuse_crypto::{Curve, ed25519::Ed25519};
use defuse_digest::{Digest, sha2::Sha256};
use defuse_nep461::{OffchainMessage, SignedMessageNep};
use digest_io::IoWrapper;

/// [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md)
/// Offchain Signing Standard
pub struct Nep413;

impl Nep413 {
    /// Verify signature over given payload for given public key according to
    /// [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md).
    #[must_use = "check if verification passed"]
    #[inline]
    pub fn verify(
        public_key: &<Ed25519 as Curve>::PublicKey,
        payload: &Nep413Payload,
        signature: &<Ed25519 as Curve>::Signature,
    ) -> bool {
        Ed25519::verify(public_key, &Self::prehash(payload), signature)
    }

    /// Derive prehash for signing.
    #[inline]
    pub fn prehash(payload: &Nep413Payload) -> [u8; 32] {
        let mut hasher = IoWrapper(Sha256::new());

        // serialize directly to hasher
        borsh::to_writer(&mut hasher, &(Self::OFFCHAIN_PREFIX_TAG, payload))
            .unwrap_or_else(|_| unreachable!());

        hasher.0.finalize().into()
    }
}

impl SignedMessageNep for Nep413 {
    /// NEP number used to derive offchain prefix tag according to
    /// [NEP-461](https://github.com/near/NEPs/pull/461).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use defuse_nep413::Nep413;
    /// use defuse_nep461::OffchainMessage;
    ///
    /// assert_eq!(Nep413::OFFCHAIN_PREFIX_TAG, 2147484061);
    /// ```
    const NEP_NUMBER: u32 = 413;
}

/// [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md) payload
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))]
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Nep413Payload {
    pub message: String,

    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::base64::Base64"))]
    pub nonce: [u8; 32],

    pub recipient: String,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub callback_url: Option<String>,
}

impl Nep413Payload {
    #[must_use]
    #[inline]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            nonce: [0u8; 32],
            recipient: String::new(),
            callback_url: None,
        }
    }

    #[must_use]
    #[inline]
    pub fn nonce(mut self, nonce: impl Into<[u8; 32]>) -> Self {
        self.nonce = nonce.into();
        self
    }

    #[must_use]
    #[inline]
    pub fn recipient(mut self, recipient: impl Display) -> Self {
        self.recipient = recipient.to_string();
        self
    }

    #[must_use]
    #[inline]
    pub fn callback_url(mut self, callback_url: impl Into<String>) -> Self {
        self.callback_url = Some(callback_url.into());
        self
    }
}

#[cfg(feature = "near-kit")]
const _: () = {
    impl From<Nep413Payload> for near_kit::nep413::SignMessageParams {
        #[inline]
        fn from(payload: Nep413Payload) -> Self {
            Self {
                message: payload.message,
                nonce: payload.nonce,
                recipient: payload.recipient,
                callback_url: payload.callback_url,
                state: None,
            }
        }
    }
};

#[cfg(test)]
mod tests {
    use defuse_crypto::ed25519::ed25519_dalek::{
        PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH, Signature, VerifyingKey,
    };
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        hex!("e2e9cb7ac57cb46d4da1ce1d1cc2c33bdfe17407c517916b522724a8ea2c6c50"),
        Nep413Payload {
            message: "Hello, world!".to_string(),
            nonce: [0u8; 32],
            recipient: "intents.near".to_string(),
            callback_url: None,
        },
        hex!("e2ff6254871a3fec1853c167b42f0f14248c4cf7fef5452dc24d8dbdc5c4bf183ab707322b4d782d5f5a05571bae476c5f7ee41c473f3002e600865e46b75d0f"),
    )]
    fn verify_ok(
        #[case] public_key: [u8; PUBLIC_KEY_LENGTH],
        #[case] payload: Nep413Payload,
        #[case] signature: [u8; SIGNATURE_LENGTH],
    ) {
        let public_key = VerifyingKey::from_bytes(&public_key).unwrap();
        let signature = Signature::from_bytes(&signature);

        assert!(Nep413::verify(&public_key, &payload, &signature));
    }
}
