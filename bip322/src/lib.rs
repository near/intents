pub mod bitcoin_minimal;
pub mod error;
pub mod hashing;
#[cfg(test)]
pub mod tests;
pub mod transaction;
pub mod verification;

use bitcoin_minimal::{Address, Bip322Witness, Transaction};
use defuse_crypto::{Curve, Payload, Secp256k1, SignedPayload};
use hashing::Bip322MessageHasher;
use near_sdk::{env, near};
use transaction::Bip322TransactionBuilder;
use serde_with::serde_as;

use crate::bitcoin_minimal::hash160;
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

    /// BIP-322 signature data as a witness stack.
    ///
    /// The witness format depends on the address type:
    /// - P2PKH/P2WPKH: [signature, pubkey]
    /// - P2SH: [signature, pubkey, redeem_script]
    /// - P2WSH: [signature, pubkey, witness_script]
    pub signature: Bip322Witness,
}

impl Payload for SignedBip322Payload {
    #[inline]
    fn hash(&self) -> near_sdk::CryptoHash {
        match &self.address {
            Address::P2PKH { .. } => self.hash_p2pkh_message(),
            Address::P2WPKH { .. } => self.hash_p2wpkh_message(),
            Address::P2SH { .. } => self.hash_p2sh_message(),
            Address::P2WSH { .. } => self.hash_p2wsh_message(),
        }
    }
}

impl SignedPayload for SignedBip322Payload {
    type PublicKey = <Secp256k1 as Curve>::PublicKey;

    fn verify(&self) -> Option<Self::PublicKey> {
        match &self.address {
            Address::P2PKH { .. } => verification::p2pkh::verify_p2pkh_signature(self),
            Address::P2WPKH { .. } => verification::p2wpkh::verify_p2wpkh_signature(self),
            Address::P2SH { .. } => verification::p2sh::verify_p2sh_signature(self),
            Address::P2WSH { .. } => verification::p2wsh::verify_p2wsh_signature(self),
        }
    }
}

impl SignedBip322Payload {
    /// Computes the BIP-322 signature hash for P2PKH addresses.
    ///
    /// P2PKH (Pay-to-Public-Key-Hash) is the original Bitcoin address format.
    /// This method implements the BIP-322 process specifically for P2PKH addresses:
    ///
    /// 1. Creates a "`to_spend`" transaction with the message hash in input script
    /// 2. Creates a "`to_sign`" transaction that spends from "`to_spend`" transaction
    /// 3. Computes the signature hash using the standard Bitcoin sighash algorithm
    ///
    /// The pubkey hash is obtained from the already-validated address stored in `self.address`.
    ///
    /// # Returns
    ///
    /// The 32-byte signature hash that should be signed according to BIP-322 for P2PKH.
    fn hash_p2pkh_message(&self) -> near_sdk::CryptoHash {
        // Step 1: Create the "to_spend" transaction
        // This transaction contains the BIP-322 message hash in its input script
        let to_spend = self.create_to_spend();

        // Step 2: Create the "to_sign" transaction
        // This transaction spends from the "to_spend" transaction
        let to_sign = Self::create_to_sign(&to_spend);

        // Step 3: Compute the final signature hash using legacy algorithm
        // P2PKH uses the original Bitcoin sighash algorithm (pre-segwit)
        Bip322MessageHasher::compute_message_hash(
            &to_spend,
            &to_sign,
            &self.address,
        )
    }

    /// Computes the BIP-322 signature hash for P2WPKH addresses.
    ///
    /// P2WPKH (Pay-to-Witness-Public-Key-Hash) is the segwit version of P2PKH.
    /// The process is similar to P2PKH but uses segwit v0 sighash computation:
    ///
    /// 1. Creates the same "`to_spend`" and "`to_sign`" transaction structure
    /// 2. Uses segwit v0 sighash algorithm instead of legacy sighash
    /// 3. The witness program contains the pubkey hash (20 bytes for v0)
    ///
    /// The witness program is obtained from the already-validated address stored in `self.address`.
    ///
    /// # Returns
    ///
    /// The 32-byte signature hash that should be signed according to BIP-322 for P2WPKH.
    fn hash_p2wpkh_message(&self) -> near_sdk::CryptoHash {
        // Step 1: Create the "to_spend" transaction (same as P2PKH)
        // The transaction structure is identical regardless of address type
        let to_spend = self.create_to_spend();

        // Step 2: Create the "to_sign" transaction (same as P2PKH)
        // The spending transaction is also identical in structure
        let to_sign = Self::create_to_sign(&to_spend);

        // Step 3: Compute signature hash using segwit v0 algorithm
        // P2WPKH uses the BIP-143 segwit sighash algorithm (not legacy)
        Bip322MessageHasher::compute_message_hash(
            &to_spend,
            &to_sign,
            &self.address,
        )
    }

    /// Computes the BIP-322 signature hash for P2SH addresses.
    ///
    /// P2SH (Pay-to-Script-Hash) addresses contain a hash of a redeem script.
    /// The BIP-322 process for P2SH is similar to P2PKH but uses legacy sighash algorithm
    /// since P2SH predates segwit.
    ///
    /// The script hash is obtained from the already-validated address stored in `self.address`.
    ///
    /// # Returns
    ///
    /// The 32-byte signature hash that should be signed according to BIP-322 for P2SH.
    fn hash_p2sh_message(&self) -> near_sdk::CryptoHash {
        // Step 1: Create the "to_spend" transaction
        // For P2SH, this contains the P2SH script_pubkey
        let to_spend = self.create_to_spend();

        // Step 2: Create the "to_sign" transaction
        // For P2SH, this will reference the to_spend output
        let to_sign = Self::create_to_sign(&to_spend);

        // Step 3: Compute signature hash using legacy algorithm
        // P2SH uses the same legacy sighash as P2PKH (not segwit)
        Bip322MessageHasher::compute_message_hash(
            &to_spend,
            &to_sign,
            &self.address,
        )
    }

    /// Computes the BIP-322 signature hash for P2WSH addresses.
    ///
    /// P2WSH (Pay-to-Witness-Script-Hash) addresses contain a SHA256 hash of a witness script.
    /// The BIP-322 process for P2WSH uses the segwit v0 sighash algorithm.
    ///
    /// The witness program is obtained from the already-validated address stored in `self.address`.
    ///
    /// # Returns
    ///
    /// The 32-byte signature hash that should be signed according to BIP-322 for P2WSH.
    fn hash_p2wsh_message(&self) -> near_sdk::CryptoHash {
        // Step 1: Create the "to_spend" transaction
        // For P2WSH, this contains the P2WSH script_pubkey (OP_0 + 32-byte script hash)
        let to_spend = self.create_to_spend();

        // Step 2: Create the "to_sign" transaction
        // For P2WSH, this will reference the to_spend output
        let to_sign = Self::create_to_sign(&to_spend);

        // Step 3: Compute signature hash using segwit v0 algorithm
        // P2WSH uses the same segwit sighash as P2WPKH (BIP-143)
        Bip322MessageHasher::compute_message_hash(
            &to_spend,
            &to_sign,
            &self.address,
        )
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
        Bip322TransactionBuilder::create_to_spend(&self.address, &message_hash)
    }

    /// Creates the \"`to_sign`\" transaction according to BIP-322 specification.
    ///
    /// The \"`to_sign`\" transaction spends from the \"`to_spend`\" transaction and represents
    /// what would actually be signed by a Bitcoin wallet. Its structure:
    ///
    /// - **Version**: 0 (BIP-322 marker, same as `to_spend`)
    /// - **Input**: Single input that spends the \"`to_spend`\" transaction:
    ///   - Previous output: TXID of `to_spend` transaction, index 0
    ///   - Script: Empty (for segwit) or minimal script (for legacy)
    ///   - Sequence: 0
    /// - **Output**: Single output with `OP_RETURN` (provably unspendable)
    /// - **Locktime**: 0
    ///
    /// The signature verification process computes the sighash of this transaction,
    /// which is what the private key actually signs.
    ///
    /// # Arguments
    ///
    /// * `to_spend` - The \"`to_spend`\" transaction created by `create_to_spend()`
    ///
    /// # Returns
    ///
    /// A `Transaction` representing the \"`to_sign`\" phase of BIP-322.
    fn create_to_sign(to_spend: &Transaction) -> Transaction {
        Bip322TransactionBuilder::create_to_sign(to_spend)
    }





    /// Try to recover public key from signature
    pub fn try_recover_pubkey(
        message_hash: &[u8; 32],
        signature_bytes: &[u8],
        expected_pubkey: &[u8],
    ) -> Option<<Secp256k1 as Curve>::PublicKey> {
        // Ensure this is a standard Bitcoin signature
        if signature_bytes.len() != 65 {
            return None;
        }

        // Calculate v byte to make it in 0-3 range
        let v = if ((signature_bytes[0] - 27) & 4) != 0 {
            // compressed
            signature_bytes[0] - 31
        } else {
            // uncompressed
            signature_bytes[0] - 27
        };

        // Secp256k1::verify(() does not work for us because of different expected format.
        // Repacking it within the contract does not look reasonable, so use env::ecrecover directly.
        env::ecrecover(message_hash, &signature_bytes[1..], v, true)
            .filter(|recovered_pubkey| recovered_pubkey.as_slice() == expected_pubkey)
    }

    /// Execute a witness script for P2WSH verification.
    ///
    /// This is a minimal implementation that only supports common P2PKH-style witness scripts
    /// used in P2WSH contexts. More complex scripts are rejected for security and simplicity.
    ///
    /// For P2WSH (Pay-to-Witness-Script-Hash), the witness script is the actual script that
    /// gets executed, while the `script_pubkey` contains the hash of this witness script.
    ///
    /// # Arguments
    ///
    /// * `witness_script` - The witness script from the witness stack
    /// * `pubkey_bytes` - The public key to validate against
    ///
    /// # Returns
    ///
    /// `true` if the script is a valid P2PKH-style witness script and the public key matches,
    /// `false` otherwise.
    ///
    /// # Supported Pattern
    ///
    /// Only supports the standard P2PKH pattern:
    /// ```text
    /// OP_DUP OP_HASH160 <20-byte-pubkey-hash> OP_EQUALVERIFY OP_CHECKSIG
    /// ```
    pub fn execute_witness_script(witness_script: &[u8], pubkey_bytes: &[u8]) -> bool {
        // For P2WSH, witness scripts can be more varied, but for BIP-322
        // we typically see P2PKH-style patterns similar to redeem scripts

        if witness_script.len() == 25 &&
            witness_script[0] == 0x76 && // OP_DUP
            witness_script[1] == 0xa9 && // OP_HASH160
            witness_script[2] == 0x14 && // Push 20 bytes
            witness_script[23] == 0x88 && // OP_EQUALVERIFY
            witness_script[24] == 0xac
        // OP_CHECKSIG
        {
            // Extract the pubkey hash from the script
            let script_pubkey_hash = &witness_script[3..23];

            // Compute HASH160 of the provided public key
            let computed_pubkey_hash = hash160(pubkey_bytes);

            // Verify the public key hash matches
            computed_pubkey_hash.as_slice() == script_pubkey_hash
        } else {
            // For now, only support simple P2PKH-style witness scripts
            // Future enhancement: full Bitcoin script interpreter
            false
        }
    }
}

