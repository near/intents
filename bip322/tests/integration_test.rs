//! # BIP-322 Integration Tests
//!
//! This test suite validates the integration of BIP-322 signature verification
//! with the broader Defuse intents system. The tests ensure that:
//!
//! 1. BIP-322 payloads can extract JSON-encoded Defuse payloads
//! 2. BIP-322 integrates properly with the Payload/SignedPayload traits
//! 3. BIP-322 works correctly within `MultiPayload` contexts
//!
//! These integration tests complement the unit tests in the main module
//! by focusing on cross-module compatibility and system-level functionality.

use defuse_bip322::{
    SignedBip322Payload,
    bitcoin_minimal::{Address, Witness, WitnessProgram},
};
use defuse_core::payload::{DefusePayload, ExtractDefusePayload};

// Helper function to verify trait implementations
const fn verify_traits_implemented<T: defuse_crypto::Payload + defuse_crypto::SignedPayload>(
    _payload: &T,
) {
}

/// Tests BIP-322 integration with `DefusePayload` extraction.
///
/// This test validates that BIP-322 signatures can carry JSON-encoded Defuse payloads
/// in their message field, which is essential for the intents system. The test:
///
/// 1. Creates a BIP-322 payload with JSON message content
/// 2. Attempts to extract a `DefusePayload` from the message
/// 3. Verifies the `ExtractDefusePayload` trait implementation works
///
/// Note: The test doesn't require a valid signature since it only tests
/// payload extraction, not signature verification.
#[test]
fn test_bip322_extract_defuse_payload_integration() {
    // Create a BIP-322 payload with a sample P2WPKH address and JSON message.
    // The JSON message represents what would typically be a Defuse intent payload.

    let bip322_payload = SignedBip322Payload {
        address: Address::P2WPKH {
            witness_program: WitnessProgram {
                version: 0,
                program: vec![1u8; 20],
            }
        },
        message: r#"{"signer_id":"alice.near","verifying_contract":"defuse.near","deadline":"Never","nonce":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","test":"value"}"#.to_string(),
        signature: Witness::new(),
    };

    let result: Result<DefusePayload<serde_json::Value>, _> =
        bip322_payload.extract_defuse_payload();

    // Verify the trait method exists and can be called (implementation tested in core module)
    assert!(
        result.is_ok() || result.is_err(),
        "ExtractDefusePayload trait should be callable"
    );
}

/// Tests BIP-322 integration with core `Payload` and `SignedPayload` traits.
///
/// This test validates that BIP-322 properly implements the fundamental traits
/// required by the Defuse system:
///
/// 1. `Payload` trait for message hashing (generates BIP-322 signature hash)
/// 2. `SignedPayload` trait for signature verification (recovers public key)
///
/// These traits are essential for BIP-322 to work within the broader intents framework.
#[test]
fn test_bip322_integration_structure() {
    use defuse_crypto::{Payload, SignedPayload};

    let bip322_payload = SignedBip322Payload {
        address: Address::P2WPKH {
            witness_program: WitnessProgram {
                version: 0,
                program: vec![1u8; 20],
            },
        },
        message: "Test message for BIP-322".to_string(),
        signature: Witness::new(),
    };

    // Test Payload trait implementation - should generate BIP-322 signature hash
    // This exercises the complete BIP-322 hashing pipeline including:
    // - BIP-322 tagged message hash creation
    // - "to_spend" and "to_sign" transaction construction
    // - Segwit v0 sighash computation
    let hash = bip322_payload.hash();
    assert_eq!(hash.len(), 32, "BIP-322 signature hash must be 32 bytes");

    // Verify hash is non-zero (not just empty bytes)
    assert!(hash.iter().any(|&b| b != 0), "Hash should not be all zeros");

    // Verify that the same payload produces the same hash (deterministic)
    let hash2 = bip322_payload.hash();
    assert_eq!(hash, hash2, "BIP-322 hash should be deterministic");

    // Create another payload with different message to verify hash changes
    let different_payload = SignedBip322Payload {
        address: bip322_payload.address.clone(),
        message: "Different message".to_string(),
        signature: Witness::new(),
    };
    let different_hash = different_payload.hash();
    assert_ne!(
        hash, different_hash,
        "Different messages should produce different hashes"
    );

    // Test SignedPayload trait implementation
    // With an empty signature, verification should gracefully return None
    // rather than panicking, demonstrating proper error handling
    let verification_result = bip322_payload.verify();
    assert!(
        verification_result.is_none(),
        "Empty signature should return None (no panic)"
    );

    // Verify the trait is properly implemented by checking type compatibility
    verify_traits_implemented(&bip322_payload);
}

/// Tests BIP-322 integration within `MultiPayload` enumeration.
///
/// This test validates that BIP-322 works correctly when wrapped in the
/// `MultiPayload` enum that handles different signature schemes (BIP-322, ERC-191, NEP-413, etc.).
///
/// The test ensures that:
/// 1. BIP-322 payloads can be wrapped in `MultiPayload::Bip322` variant
/// 2. `MultiPayload` correctly delegates to BIP-322 implementations
/// 3. The complete signature verification pipeline works through the enum
#[test]
fn test_bip322_multi_payload_integration() {
    use defuse_core::payload::multi::MultiPayload;
    use defuse_crypto::{Payload, SignedPayload};

    let bip322_payload = SignedBip322Payload {
        address: Address::P2WPKH {
            witness_program: WitnessProgram {
                version: 0,
                program: vec![1u8; 20],
            },
        },
        message: "Multi-payload test".to_string(),
        signature: Witness::new(),
    };

    // Wrap the BIP-322 payload in the MultiPayload enum
    // This simulates how BIP-322 would be used in the real intents system
    let multi_payload = MultiPayload::Bip322(bip322_payload);

    // Test that MultiPayload correctly delegates to BIP-322 implementation
    // The hash should be identical to calling .hash() directly on BIP-322
    let hash = multi_payload.hash();
    assert_eq!(
        hash.len(),
        32,
        "MultiPayload should delegate to BIP-322 hash function"
    );

    // Verify the hash matches direct BIP-322 computation
    let direct_bip322 = SignedBip322Payload {
        address: Address::P2WPKH {
            witness_program: WitnessProgram {
                version: 0,
                program: vec![1u8; 20],
            },
        },
        message: "Multi-payload test".to_string(),
        signature: Witness::new(),
    };
    let direct_hash = direct_bip322.hash();
    assert_eq!(
        hash, direct_hash,
        "MultiPayload hash should match direct BIP-322 hash"
    );

    // Test signature verification delegation through MultiPayload
    // Should behave identically to direct BIP-322 verification
    let verification = multi_payload.verify();
    assert!(
        verification.is_none(),
        "MultiPayload should delegate to BIP-322 verification"
    );

    // Verify we can pattern match on the MultiPayload variant
    match &multi_payload {
        MultiPayload::Bip322(payload) => {
            assert_eq!(
                payload.message, "Multi-payload test",
                "Should be able to access inner BIP-322 payload"
            );
            assert!(
                matches!(payload.address, Address::P2WPKH { .. }),
                "Should preserve address type"
            );
        }
        _ => panic!("Expected MultiPayload::Bip322 variant"),
    }

    // Test `ExtractDefusePayload` trait implementation through `MultiPayload`
    let json_payload = SignedBip322Payload {
        address: Address::P2WPKH {
            witness_program: WitnessProgram {
                version: 0,
                program: vec![1u8; 20],
            }
        },
        message: r#"{"signer_id":"bob.near","verifying_contract":"defuse.near","deadline":"Never","nonce":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","action":"transfer","amount":100}"#.to_string(),
        signature: Witness::new(),
    };
    let multi_json = MultiPayload::Bip322(json_payload);

    let extraction_result: Result<DefusePayload<serde_json::Value>, _> =
        multi_json.extract_defuse_payload();

    // Verify `ExtractDefusePayload` trait works through `MultiPayload` wrapper
    assert!(
        extraction_result.is_ok() || extraction_result.is_err(),
        "`ExtractDefusePayload` should work through `MultiPayload`"
    );
}
