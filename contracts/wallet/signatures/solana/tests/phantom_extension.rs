//! Real Phantom `signMessage` signatures from the display check (see
//! `provenance` in `fixtures/phantom-extension.json`).

use defuse_wallet::{RequestMessage, SignatureSchema, offchain::OffchainMessage};
use defuse_wallet_solana::{
    WalletSolana,
    crypto::{
        Curve,
        ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature},
    },
    offchain_signing_message, request_signing_message,
};
use serde_json::Value;

fn public_key(vector: &Value) -> Ed25519PublicKey {
    format!("ed25519:{}", vector["public_key"].as_str().unwrap())
        .parse()
        .unwrap()
}

/// Phantom signed exactly the captured bytes.
fn signs(key: &Ed25519PublicKey, message: &str, proof: &str) -> bool {
    let key = <Ed25519 as Curve>::PublicKey::try_from(key).unwrap();
    let signature: Ed25519Signature = proof.parse().unwrap();
    Ed25519::verify(&key, message.as_bytes(), &signature.into())
}

#[test]
fn real_phantom_signatures_verify() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/phantom-extension.json")).unwrap();
    let vectors = fixture["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 4);

    for v in vectors {
        let key = public_key(v);
        let proof = v["proof"].as_str().unwrap();
        let signed = v["message"].as_str().unwrap();
        assert!(signs(&key, signed, proof));

        // The signature covers the final LF.
        assert!(!signs(&key, signed.trim_end(), proof));

        // A proof signs only its own message.
        for other in vectors.iter().filter(|other| *other != v) {
            assert!(!signs(&key, other["message"].as_str().unwrap(), proof));
        }

        // The contract renders exactly that text and accepts the proof.
        let (rendered, verified) = if v["kind"] == "request" {
            let msg: RequestMessage = serde_json::from_value(v["msg"].clone()).unwrap();
            (
                request_signing_message(&key, &msg).unwrap(),
                WalletSolana::verify_request_msg(&key, &msg, proof),
            )
        } else {
            let msg: OffchainMessage = serde_json::from_value(v["msg"].clone()).unwrap();
            (
                offchain_signing_message(&key, &msg).unwrap(),
                WalletSolana::verify_offchain_msg(&key, &msg, proof),
            )
        };

        assert_eq!(rendered, signed);
        assert!(verified);
    }
}
