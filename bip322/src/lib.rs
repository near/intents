mod bitcoin_minimal;

use bitcoin_minimal::*;
use defuse_crypto::{Curve, Payload, Secp256k1, SignedPayload, serde::AsCurve};
use near_sdk::{near, env};
use serde_with::serde_as;

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

    // TODO:
    // * is it just signature-related bytes?
    // * or is it a serialized `to_sign` tx (pbst)?
    // * how do we differentiate between them?
    pub signature: Witness,
}

impl Payload for SignedBip322Payload {
    #[inline]
    fn hash(&self) -> near_sdk::CryptoHash {
        match self
            .address
            // TODO
            .assume_checked_ref()
            .to_address_data()
        {
            AddressData::P2pkh { pubkey_hash } => {
                // For MVP Phase 2: P2PKH support
                self.hash_p2pkh_message(&pubkey_hash)
            },
            AddressData::Segwit { witness_program } if witness_program.is_p2wpkh() => {
                // For MVP Phase 2: P2WPKH support  
                self.hash_p2wpkh_message(&witness_program)
            },
            // Phase 4: Complex address types
            AddressData::P2sh { script_hash: _ } => {
                unimplemented!("P2SH support planned for Phase 4")
            },
            AddressData::Segwit { witness_program } if witness_program.is_p2wsh() => {
                unimplemented!("P2WSH support planned for Phase 4")
            },
            _ => {
                panic!("Unsupported address type")
            },
        }
    }
}

impl SignedBip322Payload {
    /// Hash P2PKH message using NEAR SDK
    fn hash_p2pkh_message(&self, _pubkey_hash: &[u8; 20]) -> near_sdk::CryptoHash {
        let to_spend = self.create_to_spend();
        let to_sign = self.create_to_sign(&to_spend);
        self.compute_message_hash(&to_spend, &to_sign)
    }

    /// Hash P2WPKH message using NEAR SDK
    fn hash_p2wpkh_message(&self, _witness_program: &WitnessProgram) -> near_sdk::CryptoHash {
        let to_spend = self.create_to_spend();
        let to_sign = self.create_to_sign(&to_spend);
        self.compute_message_hash(&to_spend, &to_sign)
    }

    /// Create the \"to_spend\" transaction for BIP-322
    fn create_to_spend(&self) -> Transaction {
        let address = self.address.assume_checked_ref();
        let message_hash = self.compute_bip322_message_hash();
        
        Transaction {
            version: Version(0),
            lock_time: LockTime::ZERO,
            input: [TxIn {
                previous_output: OutPoint::new(Txid::all_zeros(), 0xFFFFFFFF),
                script_sig: ScriptBuilder::new()
                    .push_opcode(OP_0)
                    .push_slice(&message_hash)
                    .into_script(),
                sequence: Sequence::ZERO,
                witness: Witness::new(),
            }]
            .into(),
            output: [TxOut {
                value: Amount::ZERO,
                script_pubkey: address.script_pubkey(),
            }]
            .into(),
        }
    }

    /// Create the \"to_sign\" transaction for BIP-322
    fn create_to_sign(&self, to_spend: &Transaction) -> Transaction {
        Transaction {
            version: Version(0),
            lock_time: LockTime::ZERO,
            input: [TxIn {
                previous_output: OutPoint::new(Txid::from_byte_array(self.compute_tx_id(to_spend)), 0),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::ZERO,
                witness: Witness::new(),
            }]
            .into(),
            output: [TxOut {
                value: Amount::ZERO,
                script_pubkey: ScriptBuilder::new()
                    .push_opcode(OP_RETURN)
                    .into_script(),
            }]
            .into(),
        }
    }

    /// Compute BIP-322 tagged message hash using NEAR SDK
    fn compute_bip322_message_hash(&self) -> [u8; 32] {
        // BIP-322 uses SHA256("BIP0322-signed-message" || message)
        let tag = b"BIP0322-signed-message";
        let tag_hash = env::sha256_array(tag);
        
        // Tagged hash: SHA256(tag_hash || tag_hash || message)
        let mut input = Vec::new();
        input.extend_from_slice(&tag_hash);
        input.extend_from_slice(&tag_hash);
        input.extend_from_slice(self.message.as_bytes());
        
        env::sha256_array(&input)
    }

    /// Compute transaction ID using NEAR SDK (double SHA-256)
    fn compute_tx_id(&self, tx: &Transaction) -> [u8; 32] {
        let mut buf = Vec::new();
        tx.consensus_encode(&mut buf)
            .unwrap_or_else(|_| panic!("Transaction encoding failed"));
        
        // Double SHA-256 using NEAR SDK
        let first_hash = env::sha256_array(&buf);
        env::sha256_array(&first_hash)
    }

    /// Compute the final message hash for signature verification
    fn compute_message_hash(&self, to_spend: &Transaction, to_sign: &Transaction) -> near_sdk::CryptoHash {
        let address = self.address.assume_checked_ref();
        
        let script_code = match address.to_address_data() {
            AddressData::P2pkh { .. } => {
                &to_spend
                    .output
                    .first()
                    .expect("to_spend should have output")
                    .script_pubkey
            },
            AddressData::Segwit { witness_program } if witness_program.is_p2wpkh() => {
                &to_spend
                    .output
                    .first()
                    .expect("to_spend should have output")
                    .script_pubkey
            },
            _ => panic!("Unsupported address type in message hash computation"),
        };

        let mut sighash_cache = SighashCache::new(to_sign.clone());
        let mut buf = Vec::new();
        sighash_cache.segwit_v0_encode_signing_data_to(
            &mut buf,
            0,
            script_code,
            to_spend
                .output
                .first()
                .expect("to_spend should have output")
                .value,
            EcdsaSighashType::All,
        ).expect("Sighash encoding should succeed");
        
        // Double SHA-256 using NEAR SDK
        let first_hash = env::sha256_array(&buf);
        env::sha256_array(&first_hash)
    }

    /// Verify P2PKH signature using NEAR SDK ecrecover
    fn verify_p2pkh_signature(&self, message_hash: &[u8; 32]) -> Option<<Secp256k1 as Curve>::PublicKey> {
        // For P2PKH, we need to extract the signature from witness or script_sig
        // BIP-322 for P2PKH puts signature in witness data
        if self.signature.len() < 2 {
            return None;
        }

        // Extract signature and recovery ID from witness
        let sig_bytes = self.signature.nth(0)?;
        let recovery_id = self.signature.nth(1)?;
        
        if sig_bytes.len() != 64 || recovery_id.len() != 1 {
            return None;
        }

        let mut signature = [0u8; 64];
        signature.copy_from_slice(sig_bytes);
        let recovery_id = recovery_id[0];

        // Use NEAR SDK ecrecover
        if let Some(public_key_bytes) = env::ecrecover(message_hash, &signature, recovery_id, true) {
            // Convert to defuse-crypto public key format
            <Secp256k1 as Curve>::PublicKey::try_from(public_key_bytes.as_slice()).ok()
        } else {
            None
        }
    }

    /// Verify P2WPKH signature using NEAR SDK ecrecover  
    fn verify_p2wpkh_signature(&self, message_hash: &[u8; 32]) -> Option<<Secp256k1 as Curve>::PublicKey> {
        // P2WPKH signatures are in witness data
        if self.signature.len() < 2 {
            return None;
        }

        // Extract signature and recovery ID from witness
        let sig_bytes = self.signature.nth(0)?;
        let recovery_id = self.signature.nth(1)?;
        
        if sig_bytes.len() != 64 || recovery_id.len() != 1 {
            return None;
        }

        let mut signature = [0u8; 64];
        signature.copy_from_slice(sig_bytes);
        let recovery_id = recovery_id[0];

        // Use NEAR SDK ecrecover
        if let Some(public_key_bytes) = env::ecrecover(message_hash, &signature, recovery_id, true) {
            // Convert to defuse-crypto public key format
            <Secp256k1 as Curve>::PublicKey::try_from(public_key_bytes.as_slice()).ok()
        } else {
            None
        }
    }
}

impl SignedPayload for SignedBip322Payload {
    type PublicKey = <Secp256k1 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        // Get the message hash for this signature
        let message_hash = self.hash();
        
        // For MVP Phase 2: Support only P2PKH and P2WPKH
        let address = self.address.assume_checked_ref();
        
        match address.to_address_data() {
            AddressData::P2pkh { .. } => {
                self.verify_p2pkh_signature(&message_hash)
            },
            AddressData::Segwit { witness_program } if witness_program.is_p2wpkh() => {
                self.verify_p2wpkh_signature(&message_hash)
            },
            // Phase 4: Complex address types
            AddressData::P2sh { .. } => {
                // P2SH support planned for Phase 4
                None
            },
            AddressData::Segwit { witness_program } if witness_program.is_p2wsh() => {
                // P2WSH support planned for Phase 4
                None
            },
            _ => {
                // Unsupported address type
                None
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use near_sdk::{test_utils::VMContextBuilder, testing_env};
    use rstest::rstest;

    use super::*;

    fn setup_test_env() {
        let context = VMContextBuilder::new()
            .signer_account_id("test.near".parse().unwrap())
            .build();
        testing_env!(context);
    }

    #[test]
    fn test_gas_benchmarking_bip322_message_hash() {
        setup_test_env();
        
        let payload = SignedBip322Payload {
            address: "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l".parse().unwrap(),
            message: "Hello World".to_string(),
            signature: Witness::new(), // Empty for benchmarking
        };

        // Benchmark message hash computation
        let start_gas = env::used_gas();
        let _hash = payload.compute_bip322_message_hash();
        let hash_gas = env::used_gas().as_gas() - start_gas.as_gas();
        
        println!("BIP-322 message hash gas usage: {}", hash_gas);
        
        // Gas usage should be reasonable (less than 1M gas units)
        assert!(hash_gas < 1_000_000, "Message hash gas usage too high: {}", hash_gas);
    }

    #[test]
    fn test_gas_benchmarking_transaction_creation() {
        setup_test_env();
        
        let payload = SignedBip322Payload {
            address: "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l".parse().unwrap(),
            message: "Hello World".to_string(),
            signature: Witness::new(),
        };

        // Benchmark transaction creation
        let start_gas = env::used_gas();
        let to_spend = payload.create_to_spend();
        let tx_creation_gas = env::used_gas().as_gas() - start_gas.as_gas();
        
        println!("Transaction creation gas usage: {}", tx_creation_gas);
        
        // Benchmark transaction ID computation  
        let start_gas = env::used_gas();
        let _tx_id = payload.compute_tx_id(&to_spend);
        let tx_id_gas = env::used_gas().as_gas() - start_gas.as_gas();
        
        println!("Transaction ID computation gas usage: {}", tx_id_gas);
        
        // Gas usage should be reasonable
        assert!(tx_creation_gas < 2_000_000, "Transaction creation gas usage too high: {}", tx_creation_gas);
        assert!(tx_id_gas < 1_000_000, "Transaction ID gas usage too high: {}", tx_id_gas);
    }

    #[test]
    fn test_gas_benchmarking_p2wpkh_hash() {
        setup_test_env();
        
        let payload = SignedBip322Payload {
            address: "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l".parse().unwrap(),
            message: "Hello World".to_string(),
            signature: Witness::new(),
        };

        // Benchmark P2WPKH message hashing (full pipeline)
        let start_gas = env::used_gas();
        let _hash = payload.hash();
        let full_hash_gas = env::used_gas().as_gas() - start_gas.as_gas();
        
        println!("Full P2WPKH hash pipeline gas usage: {}", full_hash_gas);
        
        // This is the most expensive operation - should still be reasonable
        assert!(full_hash_gas < 5_000_000, "Full hash pipeline gas usage too high: {}", full_hash_gas);
    }

    #[test] 
    fn test_gas_benchmarking_ecrecover_simulation() {
        setup_test_env();
        
        // Test ecrecover gas usage with dummy data
        let message_hash = [1u8; 32];
        let signature = [2u8; 64];
        let recovery_id = 0u8;
        
        let start_gas = env::used_gas();
        // Note: This will fail but we can measure the gas cost of the call
        let _result = env::ecrecover(&message_hash, &signature, recovery_id, true);
        let ecrecover_gas = env::used_gas().as_gas() - start_gas.as_gas();
        
        println!("Ecrecover call gas usage: {}", ecrecover_gas);
        
        // Ecrecover is expensive but should be within reasonable bounds for blockchain use
        assert!(ecrecover_gas < 10_000_000, "Ecrecover gas usage too high: {}", ecrecover_gas);
    }

    #[rstest]
    #[case(
        b"",
        hex!("c90c269c4f8fcbe6880f72a721ddfbf1914268a794cbb21cfafee13770ae19f1"),
    )]
    #[case(
        b"Hello World", 
        hex!("f0eb03b1a75ac6d9847f55c624a99169b5dccba2a31f5b23bea77ba270de0a7a"),
    )]
    fn test_bip322_message_hash(#[case] message: &[u8], #[case] expected_hash: [u8; 32]) {
        setup_test_env();
        
        let payload = SignedBip322Payload {
            address: "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l".parse().unwrap(),
            message: String::from_utf8(message.to_vec()).unwrap(),
            signature: Witness::new(),
        };

        let computed_hash = payload.compute_bip322_message_hash();
        assert_eq!(computed_hash, expected_hash, "BIP-322 message hash mismatch");
    }

    #[test]
    fn test_transaction_structure() {
        setup_test_env();
        
        let payload = SignedBip322Payload {
            address: "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l".parse().unwrap(),
            message: "Hello World".to_string(),
            signature: Witness::new(),
        };

        let to_spend = payload.create_to_spend();
        let to_sign = payload.create_to_sign(&to_spend);

        // Verify transaction structure
        assert_eq!(to_spend.version, Version(0));
        assert_eq!(to_spend.input.len(), 1);
        assert_eq!(to_spend.output.len(), 1);
        
        assert_eq!(to_sign.version, Version(0));
        assert_eq!(to_sign.input.len(), 1);
        assert_eq!(to_sign.output.len(), 1);
        
        // Verify to_sign references to_spend correctly
        let to_spend_txid = payload.compute_tx_id(&to_spend);
        assert_eq!(to_sign.input[0].previous_output.txid, Txid::from_byte_array(to_spend_txid));
    }
}
