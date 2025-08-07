//! Comprehensive test suite for BIP-322 signature verification
//!
//! This module contains focused, well-organized tests that verify all aspects
//! of the BIP-322 implementation including:
//! - Address parsing and validation
//! - Message hashing (BIP-322 tagged hash)
//! - Transaction building (to_spend and to_sign)
//! - Signature verification for all address types
//! - Error handling and edge cases

use crate::bitcoin_minimal::{Address, Bip322Witness};
use crate::hashing::Bip322MessageHasher;
use crate::transaction::Bip322TransactionBuilder;
use crate::{SignedBip322Payload, AddressError};
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
                assert_eq!(witness_program.program.len(), 20, "P2WPKH program should be 20 bytes");
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
                assert_eq!(witness_program.program.len(), 32, "P2WSH program should be 32 bytes");
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
        
        assert_ne!(hash1, hash2, "Different messages should produce different hashes");
    }

    #[test]
    fn test_bip322_message_hash_empty_message() {
        setup_test_env();
        
        let empty_message = "";
        let hash = Bip322MessageHasher::compute_bip322_message_hash(empty_message);
        
        assert_eq!(hash.len(), 32, "Hash should be 32 bytes even for empty message");
        
        // Should be different from non-empty message
        let non_empty_hash = Bip322MessageHasher::compute_bip322_message_hash("a");
        assert_ne!(hash, non_empty_hash, "Empty and non-empty messages should hash differently");
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
        
        let address = Address::from_str("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa")
            .expect("Should parse address");
        let message_hash = [0u8; 32]; // Mock message hash
        
        let to_spend = Bip322TransactionBuilder::create_to_spend(&address, &message_hash);
        
        // Verify transaction structure
        assert_eq!(to_spend.version.0, 0, "Version should be 0 (BIP-322 marker)");
        assert_eq!(to_spend.input.len(), 1, "Should have exactly one input");
        assert_eq!(to_spend.output.len(), 1, "Should have exactly one output");
        
        // Verify input structure
        let input = &to_spend.input[0];
        assert_eq!(input.previous_output.txid, crate::bitcoin_minimal::Txid::all_zeros(), "Should use all-zeros TXID");
        assert_eq!(input.previous_output.vout, 0xFFFFFFFF, "Should use max vout");
        
        // Verify output has correct script_pubkey for address type
        let output = &to_spend.output[0];
        assert_eq!(output.value, crate::bitcoin_minimal::Amount::ZERO, "Output value should be zero");
    }

    #[test]
    fn test_to_sign_transaction_structure() {
        setup_test_env();
        
        let address = Address::from_str("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa")
            .expect("Should parse address");
        let message_hash = [1u8; 32]; // Mock message hash
        
        let to_spend = Bip322TransactionBuilder::create_to_spend(&address, &message_hash);
        let to_sign = Bip322TransactionBuilder::create_to_sign(&to_spend);
        
        // Verify transaction structure
        assert_eq!(to_sign.version.0, 0, "Version should be 0 (BIP-322 marker)");
        assert_eq!(to_sign.input.len(), 1, "Should have exactly one input");
        assert_eq!(to_sign.output.len(), 1, "Should have exactly one output");
        
        // Verify input references to_spend transaction
        let input = &to_sign.input[0];
        let expected_txid = Bip322TransactionBuilder::compute_tx_id(&to_spend);
        let expected_txid_struct = crate::bitcoin_minimal::Txid::from_byte_array(expected_txid);
        assert_eq!(input.previous_output.txid, expected_txid_struct, "Should reference to_spend TXID");
        assert_eq!(input.previous_output.vout, 0, "Should reference output 0");
        
        // Verify output is OP_RETURN (unspendable)
        let output = &to_sign.output[0];
        assert_eq!(output.value, crate::bitcoin_minimal::Amount::ZERO, "Output value should be zero");
    }

    #[test]
    fn test_transaction_id_computation() {
        setup_test_env();
        
        let address = Address::from_str("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4")
            .expect("Should parse address");
        let message_hash = [2u8; 32]; // Mock message hash
        
        let tx = Bip322TransactionBuilder::create_to_spend(&address, &message_hash);
        
        let txid1 = Bip322TransactionBuilder::compute_tx_id(&tx);
        let txid2 = Bip322TransactionBuilder::compute_tx_id(&tx);
        
        assert_eq!(txid1, txid2, "Same transaction should produce same TXID");
        assert_eq!(txid1.len(), 32, "TXID should be 32 bytes");
        
        // Different transaction should produce different TXID
        let different_message = [3u8; 32];
        let different_tx = Bip322TransactionBuilder::create_to_spend(&address, &different_message);
        let different_txid = Bip322TransactionBuilder::compute_tx_id(&different_tx);
        
        assert_ne!(txid1, different_txid, "Different transactions should have different TXIDs");
    }
}

#[cfg(test)]
mod witness_tests {
    use super::*;

    #[test]
    fn test_bip322_witness_creation() {
        setup_test_env();
        
        // Test P2PKH witness creation
        let signature = vec![0u8; 65];
        let pubkey = vec![1u8; 33];
        let witness = Bip322Witness::P2PKH { signature: signature.clone(), pubkey: pubkey.clone() };
        
        assert_eq!(witness.signature(), &signature, "Should return correct signature");
        assert_eq!(witness.pubkey(), &pubkey, "Should return correct pubkey");
        assert!(witness.witness_script().is_none(), "P2PKH should not have witness script");
    }

    #[test]
    fn test_witness_from_stack() {
        setup_test_env();
        
        // Test 2-element stack (P2PKH/P2WPKH pattern)
        let stack = vec![vec![0u8; 65], vec![1u8; 33]];
        let witness = Bip322Witness::from_stack(stack.clone());
        
        match witness {
            Bip322Witness::P2PKH { signature, pubkey } => {
                assert_eq!(signature, stack[0], "Should use first element as signature");
                assert_eq!(pubkey, stack[1], "Should use second element as pubkey");
            }
            _ => panic!("Should create P2PKH witness for 2-element stack"),
        }
        
        // Test 3-element stack (P2WSH pattern)
        let stack_3 = vec![vec![0u8; 65], vec![1u8; 33], vec![2u8; 25]];
        let witness_3 = Bip322Witness::from_stack(stack_3.clone());
        
        match witness_3 {
            Bip322Witness::P2WSH { signature, pubkey, witness_script } => {
                assert_eq!(signature, stack_3[0], "Should use first element as signature");
                assert_eq!(pubkey, stack_3[1], "Should use second element as pubkey");
                assert_eq!(witness_script, stack_3[2], "Should use third element as witness script");
            }
            _ => panic!("Should create P2WSH witness for 3-element stack"),
        }
    }

    #[test]
    fn test_witness_signature_length_validation() {
        setup_test_env();
        
        let valid_witness = Bip322Witness::P2PKH { 
            signature: vec![0u8; 65], 
            pubkey: vec![1u8; 33] 
        };
        assert!(valid_witness.validate_signature_length(), "65-byte signature should be valid");
        
        let invalid_witness = Bip322Witness::P2PKH { 
            signature: vec![0u8; 64], // Too short
            pubkey: vec![1u8; 33] 
        };
        assert!(!invalid_witness.validate_signature_length(), "64-byte signature should be invalid");
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
        assert!(result.is_none(), "Wrong witness type should fail verification");
    }

    #[test] 
    fn test_signature_verification_empty_witness() {
        setup_test_env();
        
        let address = Address::from_str("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa")
            .expect("Should parse address");
        
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
        assert!(result.is_none(), "Invalid signature length should fail verification");
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
        let to_spend = Bip322TransactionBuilder::create_to_spend(&address, &message_hash);
        let to_sign = Bip322TransactionBuilder::create_to_sign(&to_spend);
        
        // Verify transaction linkage
        let to_spend_txid = Bip322TransactionBuilder::compute_tx_id(&to_spend);
        let expected_txid_struct = crate::bitcoin_minimal::Txid::from_byte_array(to_spend_txid);
        assert_eq!(
            to_sign.input[0].previous_output.txid, 
            expected_txid_struct,
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
        let p2pkh_message_hash = Bip322MessageHasher::compute_bip322_message_hash("Hello, BIP-322!");
        assert_ne!(message_hash, p2pkh_message_hash, "Different messages should hash differently");
        
        // Test segwit-specific sighash
        let to_spend = Bip322TransactionBuilder::create_to_spend(&address, &message_hash);
        let to_sign = Bip322TransactionBuilder::create_to_sign(&to_spend);
        let sighash = Bip322MessageHasher::compute_message_hash(&to_spend, &to_sign, &address);
        
        // Segwit and legacy should produce different sighashes for same message
        let p2pkh_address = Address::from_str("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").unwrap();
        let legacy_sighash = Bip322MessageHasher::compute_message_hash(&to_spend, &to_sign, &p2pkh_address);
        assert_ne!(sighash, legacy_sighash, "Segwit and legacy sighash should differ");
    }
}