//! Integration tests for Sr25519 signature verification

use defuse_crypto::{Payload, SignedPayload};
use defuse_sr25519::{SignedSr25519Payload, Sr25519Payload};
use rand::thread_rng;
use schnorrkel::Keypair;

#[test]
fn test_sr25519_signature_verification() {
    // Generate a keypair
    let mut rng = thread_rng();
    let keypair = Keypair::generate_with(&mut rng);

    // Create a message
    let message = "Hello, Sr25519!";
    let context = b"substrate";

    // Sign the message
    let signature = keypair.sign_simple(context, message.as_bytes());

    // Create the signed payload
    let signed_payload = SignedSr25519Payload {
        payload: Sr25519Payload::new(message.to_string()),
        public_key: keypair.public.to_bytes(),
        signature: signature.to_bytes(),
    };

    // Verify the signature
    let verified_key = signed_payload.verify();
    assert!(verified_key.is_some(), "Signature verification failed");
    assert_eq!(
        verified_key.unwrap(),
        keypair.public.to_bytes(),
        "Recovered public key doesn't match"
    );
}

#[test]
fn test_sr25519_invalid_signature() {
    // Generate a keypair
    let mut rng = thread_rng();
    let keypair = Keypair::generate_with(&mut rng);

    // Create a message
    let message = "Hello, Sr25519!";

    // Create an invalid signature (all zeros)
    let invalid_signature = [0u8; 64];

    // Create the signed payload with invalid signature
    let signed_payload = SignedSr25519Payload {
        payload: Sr25519Payload::new(message.to_string()),
        public_key: keypair.public.to_bytes(),
        signature: invalid_signature,
    };

    // Verify should fail
    let verified_key = signed_payload.verify();
    assert!(
        verified_key.is_none(),
        "Invalid signature was incorrectly verified"
    );
}

#[test]
fn test_sr25519_different_messages() {
    // Generate a keypair
    let mut rng = thread_rng();
    let keypair = Keypair::generate_with(&mut rng);

    // Sign two different messages with the standard "substrate" context
    let message1 = "First message";
    let message2 = "Second message";
    let context = b"substrate";

    let signature1 = keypair.sign_simple(context, message1.as_bytes());
    let signature2 = keypair.sign_simple(context, message2.as_bytes());

    // Verify first signature
    let signed_payload1 = SignedSr25519Payload {
        payload: Sr25519Payload::new(message1.to_string()),
        public_key: keypair.public.to_bytes(),
        signature: signature1.to_bytes(),
    };
    assert!(
        signed_payload1.verify().is_some(),
        "First signature verification failed"
    );

    // Verify second signature
    let signed_payload2 = SignedSr25519Payload {
        payload: Sr25519Payload::new(message2.to_string()),
        public_key: keypair.public.to_bytes(),
        signature: signature2.to_bytes(),
    };
    assert!(
        signed_payload2.verify().is_some(),
        "Second signature verification failed"
    );

    // Cross-verification should fail (signature1 with message2)
    let cross_payload = SignedSr25519Payload {
        payload: Sr25519Payload::new(message2.to_string()),
        public_key: keypair.public.to_bytes(),
        signature: signature1.to_bytes(),
    };
    assert!(
        cross_payload.verify().is_none(),
        "Cross-verification should fail"
    );
}

#[test]
fn test_payload_hash_consistency() {
    let payload1 = Sr25519Payload::new("test".to_string());
    let payload2 = Sr25519Payload::new("test".to_string());

    // Same payload should produce same hash
    assert_eq!(payload1.hash(), payload2.hash());

    let payload3 = Sr25519Payload::new("different".to_string());
    // Different payload should produce different hash
    assert_ne!(payload1.hash(), payload3.hash());
}
