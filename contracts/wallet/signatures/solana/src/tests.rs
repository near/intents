use std::{cell::Cell, collections::BTreeSet};

use super::*;

#[test]
fn malformed_proof_skips_message_provider() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/solana.json")).unwrap();
    let vector = &vectors["vectors"][0];
    let key: Ed25519PublicKey = vector["public_key"].as_str().unwrap().parse().unwrap();
    let proof = vector["proof"].as_str().unwrap();

    for proof in [
        String::new(),
        "ed25519:".to_owned(),
        "ed25519:0".to_owned(),
        format!("ed25519:{}", "1".repeat(89)),
        format!("ed25519:{}", bs58::encode([0u8; 63]).into_string()),
        format!("ed25519:{}", bs58::encode([0u8; 65]).into_string()),
        proof.replacen("ed25519:", "ED25519:", 1),
        proof.replacen("ed25519:", "secp256k1:", 1),
        proof.strip_prefix("ed25519:").unwrap().to_owned(),
    ] {
        // A false result alone would not catch eager message construction.
        assert!(!WalletSolana::verify(
            &key,
            || panic!("malformed proof must not render a message"),
            &proof,
        ));
    }
}

#[test]
fn valid_proofs_cover_base58_limits_and_render_once() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/solana.json")).unwrap();
    let mut encoded_lengths = BTreeSet::new();

    for vector in vectors["vectors"].as_array().unwrap() {
        let key: Ed25519PublicKey = vector["public_key"].as_str().unwrap().parse().unwrap();
        let proof = vector["proof"].as_str().unwrap();

        encoded_lengths.insert(proof.strip_prefix("ed25519:").unwrap().len());

        let calls = Cell::new(0);

        assert!(WalletSolana::verify(
            &key,
            || {
                calls.set(calls.get() + 1);
                Ok(vector["message"].as_str().unwrap().to_owned())
            },
            proof,
        ));
        assert_eq!(calls.get(), 1);
        assert!(!WalletSolana::verify(
            &key,
            || Err(MessageError::TooLarge),
            proof
        ));
    }
    // Independent vectors cover both common widths, including the maximum.
    assert!(encoded_lengths.contains(&87));
    assert!(encoded_lengths.contains(&88));
}

#[test]
fn signature_length_and_leading_zeroes_are_checked_before_rendering() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/solana.json")).unwrap();
    let key: Ed25519PublicKey = vectors["vectors"][0]["public_key"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    for size in [63, 64, 65] {
        for zeroes in 0..=size {
            let mut bytes = vec![255; size];
            bytes[..zeroes].fill(0);
            let proof = format!("ed25519:{}", bs58::encode(bytes).into_string());
            let calls = Cell::new(0);

            // Stop before crypto: this isolates the decoded-size boundary.
            assert!(!WalletSolana::verify(
                &key,
                || {
                    calls.set(calls.get() + 1);
                    Err(MessageError::TooLarge)
                },
                &proof
            ));
            assert_eq!(
                calls.get(),
                usize::from(size == 64),
                "size={size}, zeroes={zeroes}"
            );

            if size == 64 {
                assert!(proof.len() <= 96);
            }
        }
    }
}

#[test]
fn unparseable_key_skips_message_provider() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/solana.json")).unwrap();
    let proof = vectors["vectors"][0]["proof"].as_str().unwrap();
    let invalid = (0u8..=255)
        .map(|byte| Ed25519PublicKey([byte; 32]))
        .find(|key| <Ed25519 as Curve>::PublicKey::try_from(key).is_err())
        .expect("the corpus must contain an invalid compressed point");
    assert!(!WalletSolana::verify(
        &invalid,
        || panic!("an unparseable key must not render a message"),
        proof,
    ));
}
