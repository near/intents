//! [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md)
//! Offchain Signing Standard

use core::fmt::Display;

use borsh::{BorshDeserialize, BorshSerialize};
use defuse_crypto::Curve;
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
    pub fn verify<C: Curve>(
        public_key: &C::PublicKey,
        payload: &Nep413Payload,
        signature: &C::Signature,
    ) -> bool {
        C::verify(public_key, &Self::prehash(payload), signature)
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
    use defuse_crypto::ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature};
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
        #[case] public_key: impl Into<Ed25519PublicKey>,
        #[case] payload: Nep413Payload,
        #[case] signature: impl Into<Ed25519Signature>,
    ) {
        assert!(Nep413::verify::<Ed25519>(
            &public_key.into().try_into().unwrap(),
            &payload,
            &signature.into().into()
        ));
    }
}
