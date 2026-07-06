use core::fmt::Display;

use anyhow::Context;
use borsh::{BorshDeserialize, BorshSerialize};
use defuse_digest::{Digest, sha2::Sha256};
use defuse_nep461::{OffchainMessage, SignedMessageNep};
use defuse_signature_schema::{Result, Schema};
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
    pub fn with_recipient<S>(mut self, recipient: impl Display) -> Self {
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

pub struct Nep413;

impl SignedMessageNep for Nep413 {
    const NEP_NUMBER: u32 = 413;
}

impl Schema<Nep413Payload> for Nep413 {
    type Output = [u8; 32];

    #[inline]
    fn derive(&self, payload: Nep413Payload) -> Result<Self::Output> {
        self.derive(&payload)
    }
}

impl Schema<&Nep413Payload> for Nep413 {
    type Output = [u8; 32];

    fn derive(&self, payload: &Nep413Payload) -> Result<Self::Output> {
        let mut hasher = IoWrapper(Sha256::new());

        // serialize directly to hasher
        borsh::to_writer(&mut hasher, &(Self::OFFCHAIN_PREFIX_TAG, payload)).context("borsh")?;

        Ok(hasher.0.finalize().into())
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
// TODO: tests
