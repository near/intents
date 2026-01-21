//! Sr25519 signing tests for execute_intents

use defuse::{
    core::{
        Deadline,
        crypto::PublicKey,
        intents::DefuseIntents,
        payload::{DefusePayload, multi::MultiPayload},
    },
    sandbox_ext::{
        account_manager::AccountManagerExt,
        intents::ExecuteIntentsExt,
    },
};
use defuse_crypto::Sr25519;
use defuse_sr25519::{SignedSr25519Payload, Sr25519Payload};
use hex_literal::hex;
use near_sdk::serde_json;
use rand_chacha::ChaCha20Rng;
use rand_core::SeedableRng;
use schnorrkel::{MiniSecretKey, SecretKey, context::attach_rng};

use super::super::{DefuseSignerExt, env::Env};

#[tokio::test]
async fn sr25519_empty_intents_gas_measurement() {
    let env = Env::new().await;
    let user = env.create_user().await;

    // Use a hardcoded sr25519 keypair (32-byte mini secret key)
    let mini_secret_key = MiniSecretKey::from_bytes(&hex!(
        "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
    ))
    .expect("valid mini secret key");
    let secret_key: SecretKey = mini_secret_key.expand(MiniSecretKey::ED25519_MODE);
    let public_key = secret_key.to_public();

    let pk_bytes: [u8; 32] = public_key.to_bytes();
    let sr25519_pubkey = PublicKey::Sr25519(pk_bytes);

    // Register sr25519 public key with defuse contract for the user account
    user.add_public_key(env.defuse.id(), &sr25519_pubkey)
        .await
        .unwrap();

    // Create DefusePayload with empty intents
    // Use a consistent deadline for both nonce and payload
    let deadline = Deadline::timeout(std::time::Duration::from_secs(120));
    let nonce = user.unique_nonce(&env.defuse, Some(deadline)).await.unwrap();

    let payload = DefusePayload {
        signer_id: user.id().clone(),
        verifying_contract: env.defuse.id().clone(),
        deadline,
        nonce,
        message: DefuseIntents { intents: vec![] },
    };

    let payload_json = serde_json::to_string(&payload).unwrap();

    // Sign with sr25519 using deterministic RNG (wallet wraps in <Bytes>...</Bytes>)
    let wrapped_message = format!("<Bytes>{payload_json}</Bytes>");

    // Use deterministic RNG for signing to avoid system randomness requirement
    let rng = ChaCha20Rng::from_seed([0u8; 32]);
    let ctx = schnorrkel::signing_context(Sr25519::SIGNING_CTX);
    let transcript = attach_rng(ctx.bytes(wrapped_message.as_bytes()), rng);
    let signature = secret_key.sign(transcript, &public_key);

    // Create signed payload
    let signed_payload = SignedSr25519Payload {
        payload: Sr25519Payload::new(payload_json),
        public_key: pk_bytes,
        signature: signature.to_bytes(),
    };

    // Convert to MultiPayload
    let multi_payload: MultiPayload = signed_payload.into();

    // Execute intents and measure gas
    let result = user
        .execute_intents(env.defuse.id(), [multi_payload])
        .await
        .unwrap();

    println!("Sr25519 empty intents gas burnt: {}", result.total_gas_burnt);
}
