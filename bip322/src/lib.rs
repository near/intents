pub mod bitcoin_minimal;
pub mod error;
pub mod hashing;
#[cfg(test)]
pub mod tests;
pub mod transaction;
pub mod verification;

use bitcoin_minimal::Transaction;
use defuse_crypto::{Curve, Payload, Secp256k1, SignedPayload};
use hashing::Bip322MessageHasher;
use near_sdk::{env, near};
use serde_with::serde_as;
use transaction::{create_to_sign, create_to_spend};

pub use bitcoin_minimal::Address;
pub use error::AddressError;

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

    /// Standard Bitcoin compact format signature (65 bytes).
    ///
    /// This is the signature produced by Bitcoin wallets in compact format:
    /// - 1 byte: recovery ID (27-30 for uncompressed, 31-34 for compressed)
    /// - 32 bytes: r value
    /// - 32 bytes: s value
    #[serde_as(as = "serde_with::Bytes")]
    pub signature: [u8; 65],
}

impl Payload for SignedBip322Payload {
    #[inline]
    fn hash(&self) -> near_sdk::CryptoHash {
        self.compute_bip322_hash()
    }
}

impl SignedPayload for SignedBip322Payload {
    type PublicKey = <Secp256k1 as Curve>::PublicKey;

    fn verify(&self) -> Option<Self::PublicKey> {
        match &self.address {
            Address::P2PKH { .. } => verification::verify_p2pkh_signature(self),
            Address::P2WPKH { .. } => verification::verify_p2wpkh_signature(self),
            Address::P2SH { .. } => verification::verify_p2sh_signature(self),
            Address::P2WSH { .. } => verification::verify_p2wsh_signature(self),
        }
    }
}

impl SignedBip322Payload {
    /// Computes the BIP-322 signature hash for any address type.
    ///
    /// This method implements the universal BIP-322 process:
    ///
    /// 1. Creates a "`to_spend`" transaction with the message hash in input script
    /// 2. Creates a "`to_sign`" transaction that spends from "`to_spend`" transaction  
    /// 3. Computes the signature hash using the appropriate algorithm for the address type
    ///
    /// The `Bip322MessageHasher::compute_message_hash` automatically selects the correct
    /// hashing algorithm based on the address type:
    /// - P2PKH/P2SH: Legacy Bitcoin sighash algorithm (pre-segwit)
    /// - P2WPKH/P2WSH: Segwit v0 sighash algorithm (BIP-143)
    ///
    /// # Returns
    ///
    /// The 32-byte signature hash that should be signed according to BIP-322.
    fn compute_bip322_hash(&self) -> near_sdk::CryptoHash {
        // Step 1: Create the "to_spend" transaction
        // Contains the BIP-322 message hash in its input script
        let to_spend = self.create_to_spend();

        // Step 2: Create the "to_sign" transaction
        // References the to_spend output
        let to_sign = create_to_sign(&to_spend);

        // Step 3: Compute signature hash using appropriate algorithm for address type
        Bip322MessageHasher::compute_message_hash(&to_spend, &to_sign, &self.address)
    }

    /// Creates the \"`to_spend`\" transaction according to BIP-322 specification.
    ///
    /// The \"`to_spend`\" transaction is a virtual transaction that contains the message
    /// to be signed. It follows this exact structure per BIP-322:
    ///
    /// - **Version**: 0 (special BIP-322 marker)
    /// - **Input**: Single input with:
    ///   - Previous output: All-zeros TXID, index 0xFFFFFFFF (coinbase-like)
    ///   - Script: `OP_0` + 32-byte BIP-322 tagged message hash
    ///   - Sequence: 0
    /// - **Output**: Single output with:
    ///   - Value: 0 (no actual bitcoin being spent)
    ///   - Script: The address's `script_pubkey` (P2PKH or P2WPKH)
    /// - **Locktime**: 0
    ///
    /// This transaction is never broadcast to the Bitcoin network - it's purely
    /// a construction for creating a standardized signature hash.
    ///
    /// # Returns
    ///
    /// A `Transaction` representing the \"`to_spend`\" phase of BIP-322.
    ///
    fn create_to_spend(&self) -> Transaction {
        let message_hash = Bip322MessageHasher::compute_bip322_message_hash(&self.message);
        create_to_spend(&self.address, &message_hash)
    }

    /// Try to recover public key from signature
    pub fn try_recover_pubkey(
        message_hash: &[u8; 32],
        signature_bytes: &[u8; 65],
    ) -> Option<<Secp256k1 as Curve>::PublicKey> {
        // Validate recovery ID range (27-34 for standard Bitcoin compact format)
        let recovery_id = signature_bytes[0];
        if recovery_id < 27 || recovery_id > 34 {
            return None; // Invalid recovery ID
        }

        // Calculate v byte to make it in 0-3 range
        let mut recovery_id = signature_bytes[0] - 27;
        if recovery_id >= 4 {
            recovery_id -= 4;
        }

        // Use env::ecrecover to recover public key from signature
        env::ecrecover(message_hash, &signature_bytes[1..], recovery_id, true)
    }
}
