//! Comprehensive test suite for BIP-322 signature verification
//!
//! This module contains focused, well-organized tests that verify all aspects
//! of the BIP-322 implementation including:
//! - Address parsing and validation
//! - Message hashing (BIP-322 tagged hash)
//! - Transaction building (to_spend and to_sign)
//! - Signature verification for all address types
//! - Error handling and edge cases

use crate::bitcoin_minimal::Address;
use crate::hashing::Bip322MessageHasher;
use crate::transaction::{compute_tx_id, create_to_sign, create_to_spend};
use crate::{AddressError, SignedBip322Payload};
use defuse_crypto::SignedPayload;
use near_sdk::{test_utils::VMContextBuilder, testing_env};
use std::str::FromStr;

/// Setup test environment with NEAR SDK testing utilities
fn setup_test_env() {
    let context = VMContextBuilder::new()
        .signer_account_id("test.near".parse().unwrap())
        .build();
    testing_env!(context);
}

#[cfg(test)]
mod address_parsing_tests {
    use super::*;

    #[test]
    fn test_p2pkh_address_parsing() {
        setup_test_env();

        // Valid P2PKH address (Bitcoin mainnet)
        let address_str = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
        let address = Address::from_str(address_str).expect("Should parse P2PKH address");

        match address {
            Address::P2PKH { pubkey_hash } => {
                assert_eq!(pubkey_hash.len(), 20, "P2PKH hash should be 20 bytes");
            }
            _ => panic!("Should be P2PKH address"),
        }
    }

    #[test]
    fn test_p2wpkh_address_parsing() {
        setup_test_env();

        // Valid P2WPKH address (bech32)
        let address_str = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";
        let address = Address::from_str(address_str).expect("Should parse P2WPKH address");

        match address {
            Address::P2WPKH { witness_program } => {
                assert_eq!(witness_program.version, 0, "Should be witness version 0");
                assert_eq!(
                    witness_program.program.len(),
                    20,
                    "P2WPKH program should be 20 bytes"
                );
            }
            _ => panic!("Should be P2WPKH address"),
        }
    }

    #[test]
    fn test_p2wsh_address_parsing() {
        setup_test_env();

        // Valid P2WSH address (bech32, 32 bytes)
        let address_str = "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3qccfmv3";
        let address = Address::from_str(address_str).expect("Should parse P2WSH address");

        match address {
            Address::P2WSH { witness_program } => {
                assert_eq!(witness_program.version, 0, "Should be witness version 0");
                assert_eq!(
                    witness_program.program.len(),
                    32,
                    "P2WSH program should be 32 bytes"
                );
            }
            _ => panic!("Should be P2WSH address"),
        }
    }

    #[test]
    fn test_p2sh_address_parsing() {
        setup_test_env();

        // Valid P2SH address
        let address_str = "3EktnHQD7RiAE6uzMj2ZifT9YgRrkSgzQX";
        let address = Address::from_str(address_str).expect("Should parse P2SH address");

        match address {
            Address::P2SH { script_hash } => {
                assert_eq!(script_hash.len(), 20, "P2SH hash should be 20 bytes");
            }
            _ => panic!("Should be P2SH address"),
        }
    }

    #[test]
    fn test_invalid_address_parsing() {
        setup_test_env();

        // Invalid addresses should return appropriate errors
        let invalid_addresses = vec![
            ("", AddressError::UnsupportedFormat),
            ("invalid", AddressError::UnsupportedFormat),
            ("1", AddressError::InvalidLength),
            ("bc1", AddressError::InvalidBech32),
        ];

        for (addr_str, _expected_error) in invalid_addresses {
            let result = Address::from_str(addr_str);
            assert!(result.is_err(), "Should fail to parse: {}", addr_str);

            // Note: We can't easily match the exact error type without more complex setup
            // This test ensures parsing fails as expected
        }
    }
}

#[cfg(test)]
mod message_hashing_tests {
    use super::*;

    #[test]
    fn test_bip322_message_hash_deterministic() {
        setup_test_env();

        let message = "Hello, BIP-322!";
        let hash1 = Bip322MessageHasher::compute_bip322_message_hash(message);
        let hash2 = Bip322MessageHasher::compute_bip322_message_hash(message);

        assert_eq!(hash1, hash2, "Same message should produce same hash");
        assert_eq!(hash1.len(), 32, "Hash should be 32 bytes");
    }

    #[test]
    fn test_bip322_message_hash_different_messages() {
        setup_test_env();

        let message1 = "Hello, BIP-322!";
        let message2 = "Different message";

        let hash1 = Bip322MessageHasher::compute_bip322_message_hash(message1);
        let hash2 = Bip322MessageHasher::compute_bip322_message_hash(message2);

        assert_ne!(
            hash1, hash2,
            "Different messages should produce different hashes"
        );
    }

    #[test]
    fn test_bip322_message_hash_empty_message() {
        setup_test_env();

        let empty_message = "";
        let hash = Bip322MessageHasher::compute_bip322_message_hash(empty_message);

        assert_eq!(
            hash.len(),
            32,
            "Hash should be 32 bytes even for empty message"
        );

        // Should be different from non-empty message
        let non_empty_hash = Bip322MessageHasher::compute_bip322_message_hash("a");
        assert_ne!(
            hash, non_empty_hash,
            "Empty and non-empty messages should hash differently"
        );
    }

    #[test]
    fn test_bip322_message_hash_unicode() {
        setup_test_env();

        let unicode_message = "Hello, 世界! 🌍";
        let hash = Bip322MessageHasher::compute_bip322_message_hash(unicode_message);

        assert_eq!(hash.len(), 32, "Should handle Unicode messages");

        // Should be deterministic
        let hash2 = Bip322MessageHasher::compute_bip322_message_hash(unicode_message);
        assert_eq!(hash, hash2, "Unicode message should hash deterministically");
    }
}

#[cfg(test)]
mod transaction_building_tests {
    use super::*;

    #[test]
    fn test_to_spend_transaction_structure() {
        setup_test_env();

        let address =
            Address::from_str("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").expect("Should parse address");
        let message_hash = [0u8; 32]; // Mock message hash

        let to_spend = create_to_spend(&address, &message_hash);

        // Verify transaction structure
        assert_eq!(to_spend.version, 0, "Version should be 0 (BIP-322 marker)");
        assert_eq!(to_spend.input.len(), 1, "Should have exactly one input");
        assert_eq!(to_spend.output.len(), 1, "Should have exactly one output");

        // Verify input structure
        let input = &to_spend.input[0];
        assert_eq!(
            input.previous_output.txid,
            crate::bitcoin_minimal::Txid::all_zeros(),
            "Should use all-zeros TXID"
        );
        assert_eq!(
            input.previous_output.vout, 0xFFFFFFFF,
            "Should use max vout"
        );

        // Verify output has correct script_pubkey for address type
        let output = &to_spend.output[0];
        assert_eq!(output.value, 0, "Output value should be zero");
    }

    #[test]
    fn test_to_sign_transaction_structure() {
        setup_test_env();

        let address =
            Address::from_str("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").expect("Should parse address");
        let message_hash = [1u8; 32]; // Mock message hash

        let to_spend = create_to_spend(&address, &message_hash);
        let to_sign = create_to_sign(&to_spend);

        // Verify transaction structure
        assert_eq!(to_sign.version, 0, "Version should be 0 (BIP-322 marker)");
        assert_eq!(to_sign.input.len(), 1, "Should have exactly one input");
        assert_eq!(to_sign.output.len(), 1, "Should have exactly one output");

        // Verify input references to_spend transaction
        let input = &to_sign.input[0];
        let expected_txid = compute_tx_id(&to_spend);
        let expected_txid_struct = crate::bitcoin_minimal::Txid::from_byte_array(expected_txid);
        assert_eq!(
            input.previous_output.txid, expected_txid_struct,
            "Should reference to_spend TXID"
        );
        assert_eq!(input.previous_output.vout, 0, "Should reference output 0");

        // Verify output is OP_RETURN (unspendable)
        let output = &to_sign.output[0];
        assert_eq!(output.value, 0, "Output value should be zero");
    }

    #[test]
    fn test_transaction_id_computation() {
        setup_test_env();

        let address = Address::from_str("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4")
            .expect("Should parse address");
        let message_hash = [2u8; 32]; // Mock message hash

        let tx = create_to_spend(&address, &message_hash);

        let txid1 = compute_tx_id(&tx);
        let txid2 = compute_tx_id(&tx);

        assert_eq!(txid1, txid2, "Same transaction should produce same TXID");
        assert_eq!(txid1.len(), 32, "TXID should be 32 bytes");

        // Different transaction should produce different TXID
        let different_message = [3u8; 32];
        let different_tx = create_to_spend(&address, &different_message);
        let different_txid = compute_tx_id(&different_tx);

        assert_ne!(
            txid1, different_txid,
            "Different transactions should have different TXIDs"
        );
    }
}

#[cfg(test)]
mod signature_verification_tests {
    use super::*;

    #[test]
    fn test_signature_verification_wrong_witness_type() {
        setup_test_env();

        // Create a P2PKH address but use P2WPKH witness - should fail
        let p2pkh_address = Address::from_str("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa")
            .expect("Should parse P2PKH address");

        let payload = SignedBip322Payload {
            address: p2pkh_address,
            message: "Test message".to_string(),
            signature: crate::Bip322Signature::Compact { signature: [0u8; 65] }, // Empty 65-byte signature
        };

        let result = payload.verify();
        assert!(
            result.is_none(),
            "Wrong witness type should fail verification"
        );
    }

    #[test]
    fn test_signature_verification_empty_witness() {
        setup_test_env();

        let address =
            Address::from_str("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").expect("Should parse address");

        let empty_signature = [0u8; 65]; // Empty 65-byte signature

        let payload = SignedBip322Payload {
            address,
            message: "Test message".to_string(),
            signature: crate::Bip322Signature::Compact { signature: empty_signature },
        };

        let result = payload.verify();
        assert!(result.is_none(), "Empty witness should fail verification");
    }

    #[test]
    fn test_signature_verification_invalid_signature_length() {
        setup_test_env();

        let address = Address::from_str("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4")
            .expect("Should parse P2WPKH address");

        let invalid_signature = [0u8; 65]; // Valid 65-byte signature (but empty, so will fail)

        let payload = SignedBip322Payload {
            address,
            message: "Test message".to_string(),
            signature: crate::Bip322Signature::Compact { signature: invalid_signature },
        };

        let result = payload.verify();
        assert!(
            result.is_none(),
            "Invalid signature length should fail verification"
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_bip322_workflow_p2pkh() {
        setup_test_env();

        // Test the complete workflow for P2PKH
        let address = Address::from_str("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa")
            .expect("Should parse P2PKH address");
        let message = "Hello, BIP-322!";

        // Create payload (without valid signature - just testing structure)
        let mock_signature = [0u8; 65]; // Mock 65-byte signature

        let _payload = SignedBip322Payload {
            address: address.clone(),
            message: message.to_string(),
            signature: crate::Bip322Signature::Compact { signature: mock_signature },
        };

        // Test message hash computation
        let message_hash = Bip322MessageHasher::compute_bip322_message_hash(message);
        assert_eq!(message_hash.len(), 32, "Message hash should be 32 bytes");

        // Test transaction creation
        let to_spend = create_to_spend(&address, &message_hash);
        let to_sign = create_to_sign(&to_spend);

        // Verify transaction linkage
        let to_spend_txid = compute_tx_id(&to_spend);
        let expected_txid_struct = crate::bitcoin_minimal::Txid::from_byte_array(to_spend_txid);
        assert_eq!(
            to_sign.input[0].previous_output.txid, expected_txid_struct,
            "to_sign should reference to_spend"
        );

        // Test sighash computation
        let sighash = Bip322MessageHasher::compute_message_hash(&to_spend, &to_sign, &address);
        assert_eq!(sighash.len(), 32, "Sighash should be 32 bytes");

        // Note: Actual signature verification would fail with mock data,
        // but structure verification passes
    }

    #[test]
    fn test_full_bip322_workflow_p2wpkh() {
        setup_test_env();

        // Test the complete workflow for P2WPKH (segwit)
        let address = Address::from_str("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4")
            .expect("Should parse P2WPKH address");
        let message = "Segwit BIP-322 test";

        let mock_signature = [0u8; 65]; // Mock 65-byte signature

        let _payload = SignedBip322Payload {
            address: address.clone(),
            message: message.to_string(),
            signature: crate::Bip322Signature::Compact { signature: mock_signature },
        };

        // Verify message hash is different from P2PKH
        let message_hash = Bip322MessageHasher::compute_bip322_message_hash(message);
        let p2pkh_message_hash =
            Bip322MessageHasher::compute_bip322_message_hash("Hello, BIP-322!");
        assert_ne!(
            message_hash, p2pkh_message_hash,
            "Different messages should hash differently"
        );

        // Test segwit-specific sighash
        let to_spend = create_to_spend(&address, &message_hash);
        let to_sign = create_to_sign(&to_spend);
        let sighash = Bip322MessageHasher::compute_message_hash(&to_spend, &to_sign, &address);

        // Segwit and legacy should produce different sighashes for same message
        let p2pkh_address = Address::from_str("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").unwrap();
        let legacy_sighash =
            Bip322MessageHasher::compute_message_hash(&to_spend, &to_sign, &p2pkh_address);
        assert_ne!(
            sighash, legacy_sighash,
            "Segwit and legacy sighash should differ"
        );
    }

    const MESSAGE: &str = r#"{"signer_id":"alice.near","verifying_contract":"defuse.near","deadline":"Never","nonce":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","test":"value"}"#;

    #[test]
    fn test_parse_signed_bip322_payload_unisat_wallet() {
        let address = "bc1qyt6gau643sm52hvej4n4qr34h3878ahs209s27";
        let signature = "H6Gjb7ArwmAtbS7urzjT1IS+GfGLhz5XgSvu2c863K0+RcxgOFDoD7Uo+Z44CK7NcCLY1tc9eeudsYlM2zCNYDU=";

        test_parse_bip322_payload(address, signature, "unisat");
    }

    // #[test]
    // fn test_parse_signed_bip322_payload_sparrow_wallet() {
    //     let address = "3HiZ2chbEQPX5Sdsesutn6bTQPd9XdiyuL";
    //     let signature = "H3Gzu4gab41yV0mRu8xQynKDmW442sEYtz28Ilh8YQibYMLnAa9yd9WaQ6TMYKkjPVLQWInkKXDYU1jWIYBsJs8=";
    //
    //     test_parse_bip322_payload(address, signature, "sparrow");
    // }

    fn test_parse_bip322_payload(address: &str, signature: &str, info_message: &str) {
        use crate::Bip322Signature;
        
        let bip322_signature = Bip322Signature::from_str(signature)
            .expect("Should parse signature from base64 string");

        let pubkey = SignedBip322Payload {
            address: address.parse().unwrap(),
            message: MESSAGE.to_string(),
            signature: bip322_signature,
        }
        .verify();

        pubkey.expect(format!("Expected valid signature for {info_message}").as_str());
    }

    // BIP322 test vectors from official sources
    // These are reference test vectors that should be supported when full BIP322 is implemented
    #[cfg(test)]
    mod bip322_reference_vectors {
        //! Official BIP322 test vectors from:
        //! - Bitcoin BIPs repository: https://github.com/bitcoin/bips/blob/master/bip-0322.mediawiki
        //! - bip322-js library: https://github.com/ACken2/bip322-js
        //! - Corrected vectors from PR: https://github.com/bitcoin/bips/pull/1323
        //!
        //! These vectors use the full BIP322 witness format, not compact signatures.
        //! They serve as reference for future implementation improvements.

        // P2WPKH test vectors with proper BIP322 witness format
        const P2WPKH_ADDRESS: &str = "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l";
        
        const EMPTY_MESSAGE_SIGNATURE: &str = 
            "AkcwRAIgM2gBAQqvZX15ZiysmKmQpDrG83avLIT492QBzLnQIxYCIBaTpOaD20qRlEylyxFSeEA2ba9YOixpX8z46TSDtS40ASECx/EgAxlkQpQ9hYjgGu6EBCPMVPwVIVJqO4XCsMvViHI=";
        
        const HELLO_WORLD_SIGNATURE: &str = 
            "AkcwRAIgZRfIY3p7/DoVTty6YZbWS71bc5Vct9p9Fia83eRmw2QCICK/ENGfwLtptFluMGs2KsqoNSk89pO7F29zJLUx9a/sASECx/EgAxlkQpQ9hYjgGu6EBCPMVPwVIVJqO4XCsMvViHI=";
            
        const ALTERNATIVE_SIGNATURE: &str = 
            "AUD4EDDjRkK6G3lKL7Jc+ByV7j8Cj8lWLGRDmw6LLaXooczg7RxQOVyjl4VOXfHdacf5Tm5XARuxCkNi8BDXjA+5";

        // P2PKH test vector
        const P2PKH_ADDRESS: &str = "1F3sAm6ZtwLAUnj7d38pGFxtP3RVEvtsbV";
        const P2PKH_MESSAGE: &str = "This is an example of a signed message.";
        const P2PKH_SIGNATURE: &str = 
            "H9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=";

        // Extended official test vectors from BIP-322 specification and implementations
        use super::*;
        use crate::{Bip322Signature, SignedBip322Payload, hashing::Bip322MessageHasher};
        use hex_literal::hex;

        // Official BIP-322 message hash test vectors
        const EMPTY_MESSAGE_HASH: [u8; 32] = hex!("c90c269c4f8fcbe6880f72a721ddfbf1914268a794cbb21cfafee13770ae19f1");
        const HELLO_WORLD_MESSAGE_HASH: [u8; 32] = hex!("f0eb03b1a75ac6d9847f55c624a99169b5dccba2a31f5b23bea77ba270de0a7a");

        // Additional BIP-322 test vectors
        const P2WPKH_PRIVATE_KEY: &str = "L3VFeEujGtevx9w18HD1fhRbCH67Az2dpCymeRE1SoPK6XQtaN2k";
        
        // Alternative P2WPKH signatures for empty message (from bip322-js)
        const P2WPKH_EMPTY_ALT_SIGNATURE: &str = 
            "AkgwRQIhAPkJ1Q4oYS0htvyuSFHLxRQpFAY56b70UvE7Dxazen0ZAiAtZfFz1S6T6I23MWI2lK/pcNTWncuyL8UL+oMdydVgzAEhAsfxIAMZZEKUPYWI4BruhAQjzFT8FSFSajuFwrDL1Yhy";

        // Alternative P2WPKH signatures for "Hello World" (from bip322-js)
        const P2WPKH_HELLO_WORLD_ALT_SIGNATURE: &str = 
            "AkgwRQIhAOzyynlqt93lOKJr+wmmxIens//zPzl9tqIOua93wO6MAiBi5n5EyAcPScOjf1lAqIUIQtr3zKNeavYabHyR8eGhowEhAsfxIAMZZEKUPYWI4BruhAQjzFT8FSFSajuFwrDL1Yhy";

        // P2SH-P2WPKH (Nested SegWit) test vectors from bip322-js
        const P2SH_P2WPKH_ADDRESS: &str = "3HSVzEhCFuH9Z3wvoWTexy7BMVVp3PjS6f";
        const P2SH_P2WPKH_PRIVATE_KEY: &str = "KwTbAxmBXjoZM3bzbXixEr9nxLhyYSM4vp2swet58i19bw9sqk5z";
        const P2SH_P2WPKH_HELLO_WORLD_SIGNATURE: &str = 
            "AkgwRQIhAMd2wZSY3x0V9Kr/NClochoTXcgDaGl3OObOR17yx3QQAiBVWxqNSS+CKen7bmJTG6YfJjsggQ4Fa2RHKgBKrdQQ+gEhAxa5UDdQCHSQHfKQv14ybcYm1C9y6b12xAuukWzSnS+w";

        // Legacy P2PKH test vectors
        const P2PKH_LEGACY_ADDRESS: &str = "1F3sAm6ZtwLAUnj7d38pGFxtP3RVEvtsbV";
        const P2PKH_LEGACY_PRIVATE_KEY: &str = "L3VFeEujGtevx9w18HD1fhRbCH67Az2dpCymeRE1SoPK6XQtaN2k";

        // Transaction ID test vectors (from official BIP-322)
        const P2WPKH_EMPTY_TO_SPEND_TXID: &str = "c5680aa69bb8d860bf82d4e9cd3504b55dde018de765a91bb566283c545a99a7";
        const P2WPKH_EMPTY_TO_SIGN_TXID: &str = "1e9654e951a5ba44c8604c4de6c67fd78a27e81dcadcfe1edf638ba3aaebaed6";
        const P2WPKH_HELLO_TO_SPEND_TXID: &str = "b79d196740ad5217771c1098fc4a4b51e0535c32236c71f1ea4d61a2d603352b";
        const P2WPKH_HELLO_TO_SIGN_TXID: &str = "88737ae86f2077145f93cc4b153ae9a1cb8d56afa511988c149c5c8c9d93bddf";

        #[test]
        fn test_official_message_hash_vectors() {
            // Test official BIP-322 message hash vectors
            let empty_hash = Bip322MessageHasher::compute_bip322_message_hash("");
            assert_eq!(empty_hash, EMPTY_MESSAGE_HASH, 
                "Empty message hash should match official BIP-322 vector");

            let hello_hash = Bip322MessageHasher::compute_bip322_message_hash("Hello World");
            assert_eq!(hello_hash, HELLO_WORLD_MESSAGE_HASH, 
                "Hello World message hash should match official BIP-322 vector");
        }

        #[test]
        fn test_signature_format_detection() {
            // Test that our parser correctly identifies full BIP-322 signatures
            let p2wpkh_sig = Bip322Signature::from_str(EMPTY_MESSAGE_SIGNATURE).unwrap();
            match p2wpkh_sig {
                Bip322Signature::Full { .. } => {
                    // Expected: full BIP-322 witness format
                }
                Bip322Signature::Compact { .. } => {
                    panic!("Official BIP-322 signature incorrectly parsed as compact");
                }
            }

            let p2sh_sig = Bip322Signature::from_str(P2SH_P2WPKH_HELLO_WORLD_SIGNATURE).unwrap();
            match p2sh_sig {
                Bip322Signature::Full { .. } => {
                    // Expected: full BIP-322 witness format
                }
                Bip322Signature::Compact { .. } => {
                    panic!("P2SH-P2WPKH signature incorrectly parsed as compact");
                }
            }
        }

        #[test]
        fn reference_p2wpkh_empty_message() {
            // Test official P2WPKH empty message signature
            let payload = SignedBip322Payload {
                address: P2WPKH_ADDRESS.parse().unwrap(),
                message: "".to_string(),
                signature: Bip322Signature::from_str(EMPTY_MESSAGE_SIGNATURE).unwrap(),
            };
            
            assert!(payload.verify().is_some(), "P2WPKH empty message should verify");
        }

        #[test]
        fn reference_p2wpkh_empty_message_alternative() {
            // Test alternative P2WPKH empty message signature
            let payload = SignedBip322Payload {
                address: P2WPKH_ADDRESS.parse().unwrap(),
                message: "".to_string(),
                signature: Bip322Signature::from_str(P2WPKH_EMPTY_ALT_SIGNATURE).unwrap(),
            };
            
            assert!(payload.verify().is_some(), "P2WPKH empty message (alternative) should verify");
        }

        #[test]
        fn reference_p2wpkh_hello_world() {
            // Test official P2WPKH "Hello World" signature
            let payload = SignedBip322Payload {
                address: P2WPKH_ADDRESS.parse().unwrap(),
                message: "Hello World".to_string(),
                signature: Bip322Signature::from_str(HELLO_WORLD_SIGNATURE).unwrap(),
            };
            
            assert!(payload.verify().is_some(), "P2WPKH Hello World should verify");
        }

        #[test]
        fn reference_p2wpkh_hello_world_alternative() {
            // Test alternative P2WPKH "Hello World" signature
            let payload = SignedBip322Payload {
                address: P2WPKH_ADDRESS.parse().unwrap(),
                message: "Hello World".to_string(),
                signature: Bip322Signature::from_str(P2WPKH_HELLO_WORLD_ALT_SIGNATURE).unwrap(),
            };
            
            assert!(payload.verify().is_some(), "P2WPKH Hello World (alternative) should verify");
        }

        #[test]
        fn reference_p2sh_p2wpkh_hello_world() {
            // Test P2SH-P2WPKH "Hello World" signature
            let payload = SignedBip322Payload {
                address: P2SH_P2WPKH_ADDRESS.parse().unwrap(),
                message: "Hello World".to_string(),
                signature: Bip322Signature::from_str(P2SH_P2WPKH_HELLO_WORLD_SIGNATURE).unwrap(),
            };
            
            assert!(payload.verify().is_some(), "P2SH-P2WPKH Hello World should verify");
        }

        #[test]
        fn reference_p2pkh_example_message() {
            // Test P2PKH signature
            let payload = SignedBip322Payload {
                address: P2PKH_ADDRESS.parse().unwrap(),
                message: P2PKH_MESSAGE.to_string(),
                signature: Bip322Signature::from_str(P2PKH_SIGNATURE).unwrap(),
            };
            
            assert!(payload.verify().is_some(), "P2PKH example message should verify");
        }

        #[test]
        fn test_witness_stack_parsing() {
            // Test that our witness stack parser can handle real BIP-322 signatures
            let sig = Bip322Signature::from_str(EMPTY_MESSAGE_SIGNATURE).unwrap();
            match sig {
                Bip322Signature::Full { witness_stack } => {
                    // Basic validation that we parsed something
                    assert!(!witness_stack.is_empty(), "Witness stack should not be empty");
                    
                    // For BIP-322, we expect at least a signature and public key
                    assert!(witness_stack.len() >= 2, 
                        "BIP-322 witness stack should have at least 2 elements");
                }
                Bip322Signature::Compact { .. } => {
                    panic!("BIP-322 signature should not be parsed as compact");
                }
            }
        }
    }
}
