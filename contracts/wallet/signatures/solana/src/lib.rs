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

#[cfg(test)]
mod tests;

use core::str::FromStr;

use defuse_crypto::{
    Curve,
    ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature},
};
use defuse_wallet::{RequestMessage, SignatureSchema, offchain::OffchainMessage};

/// Solana wallet `signMessage(bytes, "utf8")` (as exposed by Phantom) over the full
/// readable message.
///
/// This is deliberately incompatible with the digest-signing Ed25519 variant.
pub struct WalletSolana;

impl WalletSolana {
    fn verify(
        public_key: &Ed25519PublicKey,
        message: impl FnOnce() -> Result<String, MessageError>,
        proof: &str,
    ) -> bool {
        const PREFIX: &str = "ed25519:";
        // Base58 of 64 bytes takes at most 88 characters, excluding the prefix.
        const MAX_PROOF_LEN: usize = PREFIX.len() + 88;

        // Bound decoding before rendering, for both execution and offchain proofs.
        if proof.len() > MAX_PROOF_LEN || !proof.starts_with(PREFIX) {
            return false;
        }

        let (Ok(key), Ok(signature)) = (
            <Ed25519 as Curve>::PublicKey::try_from(public_key),
            Ed25519Signature::from_str(proof),
        ) else {
            return false;
        };

        message().is_ok_and(|message| Ed25519::verify(&key, message.as_bytes(), &signature.into()))
    }
}

impl SignatureSchema for WalletSolana {
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
