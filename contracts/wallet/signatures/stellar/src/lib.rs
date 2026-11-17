#![doc = include_str!("../README.md")]

#[cfg(feature = "contract")]
mod contract;
mod message;
#[cfg(feature = "signer")]
mod signer;

pub use self::message::*;
#[cfg(feature = "signer")]
pub use self::signer::*;
pub use defuse_crypto as crypto;

use base64::{Engine, engine::general_purpose::STANDARD};
use defuse_crypto::{
    Curve,
    ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature},
};
use defuse_sep53::Sep53;
use defuse_wallet::{RequestMessage, SignatureSchema, offchain::OffchainMessage};

/// Length of a 64-byte signature in padded base64.
const PROOF_LEN: usize = 88;

/// Stellar wallet `signMessage` ([SEP-53]) over the full readable message.
///
/// This is deliberately incompatible with the digest-signing Ed25519 variant.
///
/// [SEP-53]: https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0053.md
pub struct WalletStellar;

impl WalletStellar {
    fn verify(
        public_key: &Ed25519PublicKey,
        message: impl FnOnce() -> Result<String, MessageError>,
        proof: &str,
    ) -> bool {
        // Reject other proofs before rendering the message.
        if proof.len() != PROOF_LEN {
            return false;
        }
        let Ok(key) = <Ed25519 as Curve>::PublicKey::try_from(public_key) else {
            return false;
        };
        let Some(signature) = STANDARD
            .decode(proof)
            .ok()
            .and_then(|bytes| <[u8; 64]>::try_from(bytes).ok())
        else {
            return false;
        };
        let Ok(message) = message() else {
            return false;
        };
        Sep53::verify(&key, message, &Ed25519Signature(signature).into())
    }
}

impl SignatureSchema for WalletStellar {
    type PublicKey = Ed25519PublicKey;

    fn verify_request_msg(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool {
        Self::verify(
            public_key,
            || request_signing_message(public_key, msg),
            proof,
        )
    }

    fn verify_offchain_msg(
        public_key: &Self::PublicKey,
        msg: &OffchainMessage,
        proof: &str,
    ) -> bool {
        Self::verify(
            public_key,
            || offchain_signing_message(public_key, msg),
            proof,
        )
    }
}
