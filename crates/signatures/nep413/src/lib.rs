use core::fmt::Display;

use anyhow::Context;
use borsh::{BorshDeserialize, BorshSerialize};
use defuse_digest::{Digest, sha2::Sha256};
use defuse_kdf_crypto::Ed25519;
use defuse_nep461::{OffchainMessage, SignedMessageNep};
use defuse_signature_schema::{Result, Schema, SignatureSchema};
use digest_io::IoWrapper;

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    serde(rename_all = "camelCase")
)]
#[derive(BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))]
/// [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md)
/// Offchain Signing Standard
#[derive(Debug, Clone)]
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
    pub fn with_nonce(mut self, nonce: impl Into<[u8; 32]>) -> Self {
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
    pub fn with_callback_url(mut self, callback_url: String) -> Self {
        self.callback_url = Some(callback_url);
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

#[derive(Debug, Clone, Copy, Default)]
pub struct Nep413;

impl SignedMessageNep for Nep413 {
    const NEP_NUMBER: u32 = 413;
}

impl Schema<Nep413Payload> for Nep413 {
    type Output = [u8; 32];

    /// Derive hash to sign
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hex_literal::hex;
    /// use defuse_nep413::{Nep413, Nep413Payload};
    /// use defuse_signature_schema::Schema;
    ///
    /// assert_eq!(
    ///     Nep413.derive(Nep413Payload {
    ///         message: "Hello world!".to_string(),
    ///         nonce: [0u8; 32],
    ///         recipient: "recipient".to_string(),
    ///         callback_url: None,
    ///     }).unwrap(),
    ///     hex!("41664e86aaff9224c16b88efbb5897a8b69593cac8f4ddc99fbd6400bee932ca"),
    /// );
    /// ```
    #[inline]
    fn derive(&self, payload: Nep413Payload) -> Result<Self::Output> {
        let mut hasher = IoWrapper(Sha256::new());

        // serialize directly to hasher
        borsh::to_writer(&mut hasher, &(Self::OFFCHAIN_PREFIX_TAG, payload)).context("borsh")?;

        Ok(hasher.0.finalize().into())
    }
}

impl SignatureSchema<Nep413Payload> for Nep413 {
    type Curve = Ed25519;
}

#[cfg(test)]
mod tests {
    use defuse_kdf_crypto::ed25519_dalek::{
        PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH, Signature, VerifyingKey,
    };
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b8"),
        Nep413Payload {
            message: "Hello world!".to_string(),
            nonce: [0u8; 32],
            recipient: "recipient".to_string(),
            callback_url: None,
         },
        hex!("7800a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e47"),
    )]
    fn verify_ok(
        #[case] public_key: [u8; PUBLIC_KEY_LENGTH],
        #[case] msg: Nep413Payload,
        #[case] signature: [u8; SIGNATURE_LENGTH],
    ) {
        let public_key = VerifyingKey::from_bytes(&public_key).unwrap();
        let signature = Signature::from_bytes(&signature.into());

        assert!(Nep413.verify(&public_key, msg, &signature).unwrap());
    }
}

// TODO: tests
