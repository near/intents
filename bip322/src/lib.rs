pub mod bitcoin_minimal;
pub mod error;
pub mod hashing;
pub mod signature;
#[cfg(test)]
pub mod tests;
pub mod transaction;
pub mod verification;

use defuse_crypto::{Curve, Payload, Secp256k1, SignedPayload};
use near_sdk::near;
use serde_with::serde_as;
use std::str::FromStr;

pub use bitcoin_minimal::Address;
pub use error::AddressError;
pub use signature::{Bip322Error, Bip322Signature};

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[serde(rename_all = "snake_case")]
#[derive(Debug, Clone)]
/// [BIP-322](https://github.com/bitcoin/bips/blob/master/bip-0322.mediawiki)
pub struct SignedBip322Payload {
    pub address: Address,
    pub message: String,

    /// BIP-322 signature in either compact or full format.
    ///
    /// The signature format depends on the wallet implementation:
    /// - Compact: 65-byte ECDSA signature with recovery byte (legacy format)
    /// - Full: Complete BIP-322 witness stack with transaction data
    pub signature: Bip322Signature,
}

impl Payload for SignedBip322Payload {
    #[inline]
    fn hash(&self) -> near_sdk::CryptoHash {
        self.signature
            .compute_message_hash(&self.message, &self.address)
    }
}

impl SignedPayload for SignedBip322Payload {
    type PublicKey = <Secp256k1 as Curve>::PublicKey;

    fn verify(&self) -> Option<Self::PublicKey> {
        let message_hash = self
            .signature
            .compute_message_hash(&self.message, &self.address);
        self.signature
            .extract_public_key(&message_hash, &self.address)
    }
}

impl SignedBip322Payload {
    /// Creates a SignedBip322Payload with a compact signature format.
    ///
    /// This is a convenience constructor for the most common case where
    /// wallets provide base64-encoded 65-byte signatures.
    pub fn with_compact_signature(
        address: Address,
        message: String,
        signature_base64: &str,
    ) -> Result<Self, Bip322Error> {
        let signature = Bip322Signature::from_str(signature_base64)?;
        Ok(SignedBip322Payload {
            address,
            message,
            signature,
        })
    }
}
