//! Shared NEP-641 [`AuthMessage`] test vectors.
//!
//! The fixture file is also consumed by the `near-connect-passkey` executor
//! (vitest) to lock the exact wire format across implementations. Any change
//! to these vectors is a breaking change to the NEP-641 authorization format
//! of the wallet contract.

use defuse_wallet::AuthMessage;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Fixtures {
    vectors: Vec<Vector>,
}

#[derive(Debug, Deserialize)]
struct Vector {
    name: String,
    message: AuthMessage,
    #[serde(with = "hex::serde")]
    hash: [u8; 32],
}

#[test]
fn nep641_auth_vectors() {
    let fixtures: Fixtures = serde_json::from_str(include_str!("fixtures/nep641-auth.json"))
        .expect("fixtures must deserialize as AuthMessage");

    assert!(!fixtures.vectors.is_empty());

    for Vector {
        name,
        message,
        hash,
    } in fixtures.vectors
    {
        assert_eq!(
            message.hash(),
            hash,
            "hash mismatch for vector '{name}': got {}",
            hex::encode(message.hash()),
        );

        // JSON round-trip must be lossless
        let json = serde_json::to_value(&message).unwrap();
        assert_eq!(
            serde_json::from_value::<AuthMessage>(json).unwrap(),
            message,
            "JSON round-trip mismatch for vector '{name}'",
        );
    }
}
