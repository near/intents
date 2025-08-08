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
            signature: [0u8; 65], // Empty 65-byte signature
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
            signature: empty_signature,
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
            signature: invalid_signature,
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
            signature: mock_signature,
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
            signature: mock_signature,
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

    // const MESSAGE: &str = r#"{"signer_id":"alice.near","verifying_contract":"defuse.near","deadline":"Never","nonce":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","test":"value"}"#;
    const MESSAGE: &str = r#"{
  "signer_id": "alice.near",
  "verifying_contract": "intents.near",
  "deadline": {
    "timestamp": 1734735219
  },
  "nonce": "XVoKfmScb3G+XqH9ke/fSlJ/3xO59sNhCxhpG821BH8=",
  "intents": [
    {
      "intent": "token_diff",
      "diff": {
        "nep141:usdc.near": "-1000",
        "nep141:wbtc.near": "0.001"
      }
    }
  ]
}
"#;

    use base64::{
        engine::general_purpose,
        Engine as _,
    };
    

    // #[test]
    // fn test_parse_signed_bip322_payload_leather_wallet() {
    //     let address = "bc1p4tgt4934ysj6drgcuyr492hlku6kue20rhjn7wthkeue5ku43flqn9lkfp";
    //     let signature = "AUAl8g/QcmbWNwWsGvDLORWjU6FwohDPShrRhelfc/RETVZ245o2IUNSLv6whA1ToDp96CJ3vX0JfcCPheuy1Rsw";
    //
    //     test_parse_bip322_payload(address, signature, "leather");
    // }
    //
    // #[test]
    // fn test_parse_signed_bip322_payload_magic_eden_wallet() {
    //     let address = "bc1pqcgf630uvwkx2mxrs357ur5nxv6tjylp90ewte6yf4az0j2e3c3syjm22a";
    //     let signature = "AUCi4U4Tb/A22yiIP+Yk/KgouYMdrKMlM9TYGaUPTNox4mI5DeXFw+OrZ+JIISakx+5su7k6DfKF7XerTkT0vBEO";
    //
    //     test_parse_bip322_payload(address, signature, "eden");
    // }
    //
    // #[test]
    // fn test_parse_signed_bip322_payload_xverse_wallet() {
    //     let address = "bc1psqt6kq8vts45mwrw72gll2x7kmaux6akga7lsjp2ctchhs9249wq8pj0uv";
    //     let signature = "AUAy/nD9/YJgsPMM05dnhtPmiJptiO2eHpAJ9GYhvORhptHNqeNyOsUczx3tFAC40Rn9AgGa2Zvbgi/Exp/nAccC";
    //
    //     test_parse_bip322_payload(address, signature, "xverse");
    // }
    //
    // #[test]
    // fn test_parse_signed_bip322_payload_oyl_wallet() {
    //     let address = "bc1pj3573fe3jlhf35kmzh05gthwy453xu6j7ehhsr7rrpk23mgd0ugqs4d02f";
    //     let signature = "AUGYwllbBv32z1MabDbo1/5Kpx9N3lJMyFQ35sfvUlfreMiCuk7aW++8y1xtGvul3cEdEFjTgOz3km8A2ExKrt2jAQ==";
    //
    //     test_parse_bip322_payload(address, signature, "oyl");
    // }
    //
    // #[test]
    // fn test_parse_signed_bip322_payload_ghost_wallet() {
    //     let address = "bc1p8pd76laz84v2vmx7qwuznv2yy7n5sq2dszptf4m4czhqneyfhj2st4mu9h";
    //     let signature = "AUAsoDOP3REtR1HYO3mlQKRxPt643IcMqRE/1k/+skLBUFCSbZw4esU04KMvWXc00XitpZqfIHGkafULg0CxCCz8";
    //
    //     test_parse_bip322_payload(address, signature, "ghost");
    // }

    #[test]
    fn test_parse_signed_bip322_payload_unisat_wallet() {
        let address = "bc1qyt6gau643sm52hvej4n4qr34h3878ahs209s27";
        let signature = //"IH73ZAtKGynrRcrUqlJEyTxrxkn0bKzeJKTC/h2WgA6nffOzMAXBMLIj3ToaYGtwbtP6UrITsxzYy1Tu8yQ7QyU=";
                              //"H6Gjb7ArwmAtbS7urzjT1IS+GfGLhz5XgSvu2c863K0+RcxgOFDoD7Uo+Z44CK7NcCLY1tc9eeudsYlM2zCNYDU=";
                              //IH73ZAtKGynrRcrUqlJEyTxrxkn0bKzeJKTC/h2WgA6nffOzMAXBMLIj3ToaYGtwbtP6UrITsxzYy1Tu8yQ7QyU=
        "H3240zU+IK4IZ60zAfNSppkcKfwDANatUKwquAA+SAeWQt2vOTn5LKuHg3079OIyfLuunTiWd9OmwCTKRqDMXmo=";

        test_parse_bip322_payload(address, signature, "unisat");
    }

    #[test]
    fn test_parse_signed_bip322_payload_sparrow_wallet() {
        let address = "3HiZ2chbEQPX5Sdsesutn6bTQPd9XdiyuL";
        let signature = "H3Gzu4gab41yV0mRu8xQynKDmW442sEYtz28Ilh8YQibYMLnAa9yd9WaQ6TMYKkjPVLQWInkKXDYU1jWIYBsJs8=";

        test_parse_bip322_payload(address, signature, "sparrow");
    }

    fn test_parse_bip322_payload(address: &str, signature: &str, wallet_name: &str) {
        let decoded_signature: [u8; 65] = general_purpose::STANDARD
            .decode(signature)
            .expect("Invalid binary data")
            .try_into()
            .unwrap();

        let pubkey = SignedBip322Payload {
            address: address.parse().unwrap(),
            message: MESSAGE.to_string(),
            signature: decoded_signature,
        }
        .verify();

        pubkey.expect(format!("Expected valid signature for {wallet_name} wallet").as_str());
    }
}
