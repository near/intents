//! Shared EIP-712 wallet-contract test vectors.
//!
//! The fixture file is meant to be consumed by clients implementing the
//! `eth_signTypedData_v4` flow as well, to lock the exact typed data and
//! proof format across implementations. Any change to these vectors is a
//! breaking change to the wire format of the `wallet-eip712` contract
//! variant.

use std::str::FromStr;

use defuse_wallet::{RequestMessage, SignatureSchema, offchain::OffchainMessage};
use defuse_wallet_eip712::{
    DOMAIN, Eip712AuthMessage, Eip712Message, Eip712RequestMessage, EthAddress, WalletEip712,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Fixtures {
    #[serde(with = "hex::serde")]
    domain_separator: [u8; 32],
    domain: Domain,
    address: String,
    request_vectors: Vec<Vector<RequestMessage, Eip712RequestMessage>>,
    auth_vectors: Vec<Vector<OffchainMessage, Eip712AuthMessage>>,
}

#[derive(Debug, Deserialize)]
struct Domain {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct Vector<M, T> {
    name: String,
    /// The canonical message the contract acts upon.
    message: M,
    /// The `message` of the `eth_signTypedData_v4` request.
    typed_message: T,
    /// The digest signed by the Ethereum wallet.
    #[serde(with = "hex::serde")]
    prehash: [u8; 32],
    /// The `proof` submitted on-chain.
    proof: serde_json::Value,
}

#[test]
fn eip712_wallet_message_vectors() {
    let Fixtures {
        domain_separator,
        domain,
        address,
        request_vectors,
        auth_vectors,
    } = serde_json::from_str(include_str!("fixtures/eip712-wallet-message.json"))
        .expect("fixtures must deserialize");

    assert_eq!(domain.name, DOMAIN.name);
    assert_eq!(domain.version, DOMAIN.version);
    assert_eq!(DOMAIN.separator(), domain_separator);

    let address = EthAddress::from_str(&address).unwrap();

    assert!(!request_vectors.is_empty());
    assert!(!auth_vectors.is_empty());

    for Vector {
        name,
        message,
        typed_message,
        prehash,
        proof,
    } in request_vectors
    {
        assert_eq!(
            Eip712RequestMessage::from(&message),
            typed_message,
            "typed data mismatch for vector '{name}'",
        );
        assert_eq!(
            typed_message.prehash(),
            prehash,
            "prehash mismatch for vector '{name}': got {}",
            hex::encode(typed_message.prehash()),
        );
        assert!(
            WalletEip712::verify_request_msg(&address, &message, &proof.to_string()),
            "proof of vector '{name}' didn't pass verification",
        );
    }

    for Vector {
        name,
        message,
        typed_message,
        prehash,
        proof,
    } in auth_vectors
    {
        assert_eq!(
            Eip712AuthMessage::from(&message),
            typed_message,
            "typed data mismatch for vector '{name}'",
        );
        assert_eq!(
            typed_message.prehash(),
            prehash,
            "prehash mismatch for vector '{name}': got {}",
            hex::encode(typed_message.prehash()),
        );
        assert!(
            WalletEip712::verify_offchain_msg(&address, &message, &proof.to_string()),
            "proof of vector '{name}' didn't pass verification",
        );
    }
}
