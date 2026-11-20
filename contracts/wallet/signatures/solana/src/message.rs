use defuse_crypto::ed25519::Ed25519PublicKey;
use defuse_wallet::{RequestMessage, offchain::OffchainMessage};

pub use defuse_wallet_readable::{
    MAX_SIGNING_MESSAGE_BYTES, MessageError, OFFCHAIN_PREFIX, REQUEST_PREFIX,
};

/// Reconstruct the readable v1 execution message for this Solana key.
pub fn request_signing_message(
    public_key: &Ed25519PublicKey,
    msg: &RequestMessage,
) -> Result<String, MessageError> {
    let public_key = bs58::encode(public_key.0).into_string();
    defuse_wallet_readable::request_signing_message(&[("solana_public_key", &public_key)], msg)
}

/// Reconstruct all five NEP-641 fields, preserving the exact payload string.
pub fn offchain_signing_message(
    public_key: &Ed25519PublicKey,
    msg: &OffchainMessage,
) -> Result<String, MessageError> {
    let public_key = bs58::encode(public_key.0).into_string();
    defuse_wallet_readable::offchain_signing_message(&[("solana_public_key", &public_key)], msg)
}
