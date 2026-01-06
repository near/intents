mod condvar_ext;
mod escrow_proxy_ext;
mod escrow_swap_ext;
mod multi_token_receiver;

pub use condvar_ext::{OneshotCondVarAccountExt, State};
pub use escrow_proxy_ext::EscrowProxyExt;
pub use escrow_swap_ext::EscrowSwapAccountExt;
pub use multi_token_receiver::{MT_RECEIVER_STUB_WASM, MtReceiverStubAccountExt};

use defuse_crypto::Payload;
use defuse_deadline::Deadline;
use defuse_nep413::{Nep413Payload, SignedNep413Payload};
use near_sdk::AccountId;

// ============================================================================
// Utility Functions
// ============================================================================

/// Sign a message with Ed25519 using a raw 32-byte secret key.
/// Returns (`public_key`, `signature`) as raw byte arrays.
pub fn sign_ed25519(secret_key: &[u8; 32], message: &[u8]) -> ([u8; 32], [u8; 64]) {
    use ed25519_dalek::{Signer, SigningKey};
    let signing_key = SigningKey::from_bytes(secret_key);
    let public_key = signing_key.verifying_key().to_bytes();
    let signature = signing_key.sign(message).to_bytes();
    (public_key, signature)
}

/// Get the public key for the given secret key
pub fn public_key_from_secret(secret_key: &[u8; 32]) -> defuse_crypto::PublicKey {
    let (pk, _) = sign_ed25519(secret_key, &[]);
    defuse_crypto::PublicKey::Ed25519(pk)
}

/// Sign intents using NEP-413 standard.
/// Returns a `MultiPayload` ready to be passed to `execute_intents`.
pub fn sign_intents(
    signer_id: &AccountId,
    secret_key: &[u8; 32],
    defuse_contract_id: &AccountId,
    nonce: [u8; 32],
    intents: Vec<defuse_core::intents::Intent>,
) -> defuse_core::payload::multi::MultiPayload {
    use defuse_core::intents::DefuseIntents;
    use defuse_core::payload::multi::MultiPayload;
    use defuse_core::payload::nep413::Nep413DefuseMessage;

    let deadline = Deadline::timeout(std::time::Duration::from_secs(120));

    let nep413_message = Nep413DefuseMessage {
        signer_id: signer_id.clone(),
        deadline,
        message: DefuseIntents { intents },
    };

    let nep413_payload = Nep413Payload::new(serde_json::to_string(&nep413_message).unwrap())
        .with_recipient(defuse_contract_id)
        .with_nonce(nonce);

    let hash = nep413_payload.hash();
    let (public_key, signature) = sign_ed25519(secret_key, &hash);

    MultiPayload::Nep413(SignedNep413Payload {
        payload: nep413_payload,
        public_key,
        signature,
    })
}
