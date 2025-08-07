//! BIP-322 transaction building logic
//!
//! This module contains the transaction construction methods for BIP-322 signature verification.
//! BIP-322 uses a two-transaction approach: "to_spend" and "to_sign" transactions that simulate
//! the Bitcoin signing process without requiring actual UTXOs.

use crate::bitcoin_minimal::{
    Address, Encodable, NearDoubleSha256, OP_0, OP_RETURN, OutPoint, ScriptBuf, Transaction,
    TransactionWitness, TxIn, TxOut, Txid,
};
use digest::Digest;

/// Creates the "to_spend" transaction according to BIP-322 specification.
///
/// The "to_spend" transaction is a virtual transaction that represents spending from
/// a coinbase-like output. Its structure:
///
/// - **Version**: 0 (BIP-322 marker)
/// - **Input**: Single input from virtual coinbase (all-zeros TXID, max index)
/// - **Output**: Single output with the address's script_pubkey
/// - **Locktime**: 0
///
/// # Arguments
///
/// * `address` - The Bitcoin address being verified
/// * `message_hash` - The BIP-322 tagged hash of the message
///
/// # Returns
///
/// A `Transaction` representing the "to_spend" phase of BIP-322.
pub fn create_to_spend(address: &Address, message_hash: &[u8; 32]) -> Transaction {
    Transaction {
        // Version 0 is a BIP-322 marker (normal Bitcoin transactions use version 1 or 2)
        version: 0,

        // No timelock constraints
        lock_time: 0,

        // Single input that "spends" from a virtual coinbase-like output
        input: [TxIn {
            // Previous output points to all-zeros TXID with max index (coinbase pattern)
            // This indicates this is not spending a real UTXO
            previous_output: OutPoint::new(Txid::all_zeros(), 0xFFFFFFFF),

            // Script contains OP_0 followed by the BIP-322 message hash
            // This embeds the message directly into the transaction structure
            script_sig: {
                let mut script = Vec::with_capacity(34); // 2 opcodes + 32 bytes message hash
                script.push(OP_0); // Push empty stack item
                script.push(32); // Push 32 bytes
                script.extend_from_slice(message_hash); // Push the 32-byte message hash
                ScriptBuf::from_bytes(script)
            },

            // Standard sequence number
            sequence: 0,

            // Empty witness stack (will be populated in "to_sign" transaction)
            witness: TransactionWitness::new(),
        }]
        .into(),

        // Single output that can be "spent" by the claimed address
        output: [TxOut {
            // Zero value - no actual bitcoin is involved
            value: 0,

            // The script_pubkey corresponds to the address type:
            // - P2PKH: `OP_DUP OP_HASH160 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG`
            // - P2WPKH: `OP_0 <20-byte-pubkey-hash>`
            script_pubkey: address.script_pubkey(),
        }]
        .into(),
    }
}

/// Creates the "to_sign" transaction according to BIP-322 specification.
///
/// The "to_sign" transaction spends from the "to_spend" transaction and represents
/// what would actually be signed by a Bitcoin wallet. Its structure:
///
/// - **Version**: 0 (BIP-322 marker, same as `to_spend`)
/// - **Input**: Single input that spends the "to_spend" transaction:
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
/// * `to_spend` - The "to_spend" transaction created by `create_to_spend()`
///
/// # Returns
///
/// A `Transaction` representing the "to_sign" phase of BIP-322.
pub fn create_to_sign(to_spend: &Transaction) -> Transaction {
    Transaction {
        // Version 0 to match BIP-322 specification
        version: 0,

        // No timelock constraints
        lock_time: 0,

        // Single input that spends from the "to_spend" transaction
        input: [TxIn {
            // Reference the "to_spend" transaction by its computed TXID
            // Index 0 refers to the first (and only) output of "to_spend"
            previous_output: OutPoint::new(Txid::from_byte_array(compute_tx_id(to_spend)), 0),

            // Empty script_sig (modern Bitcoin uses witness data for signatures)
            script_sig: ScriptBuf::new(),

            // Standard sequence number
            sequence: 0,

            // Empty witness (actual signature would go here in real Bitcoin)
            witness: TransactionWitness::new(),
        }]
        .into(),

        // Single output that is provably unspendable (OP_RETURN)
        output: [TxOut {
            // Zero value output
            value: 0,

            // OP_RETURN makes this output provably unspendable
            // This ensures the transaction could never be broadcast profitably
            script_pubkey: {
                let mut script = Vec::with_capacity(1); // Single OP_RETURN opcode
                script.push(OP_RETURN);
                ScriptBuf::from_bytes(script)
            },
        }]
        .into(),
    }
}

/// Computes the transaction ID (TXID) by double SHA256 hashing the serialized transaction.
///
/// This follows Bitcoin's standard transaction ID computation:
/// TXID = SHA256(SHA256(serialized_transaction))
///
/// # Arguments
///
/// * `tx` - The transaction to compute TXID for
///
/// # Returns
///
/// The 32-byte TXID as a byte array
pub fn compute_tx_id(tx: &Transaction) -> [u8; 32] {
    // Estimate for typical BIP-322 transaction: ~200-300 bytes
    let mut buf = Vec::with_capacity(300);
    tx.consensus_encode(&mut buf)
        .unwrap_or_else(|_| panic!("Transaction encoding failed"));
    NearDoubleSha256::digest(&buf).into()
}
