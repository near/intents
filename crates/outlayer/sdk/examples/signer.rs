use std::io;

use defuse_outlayer_sdk::{
    host::crypto::{ed25519, secp256k1},
    serde::{Deserialize, Serialize},
    serde_json,
    serde_with::{hex::Hex, serde_as},
};

fn main() {
    let input: Input = serde_json::from_reader(io::stdin()).expect("input: JSON");

    let output = run(input);

    serde_json::to_writer(io::stdout(), &output).expect("output: JSON");
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Input {
    Ed25519 {
        path: String,
        #[serde_as(as = "Hex")]
        msg: Vec<u8>,
    },
    Secp256k1 {
        path: String,
        #[serde_as(as = "Hex")]
        prehash: [u8; 32],
    },
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "curve", rename_all = "snake_case")]
pub enum Output {
    Ed25519 {
        #[serde_as(as = "Hex")]
        derived_pk: ed25519::PublicKey,
        #[serde_as(as = "Hex")]
        signature: ed25519::Signature,
    },
    Secp256k1 {
        #[serde_as(as = "Hex")]
        derived_pk: secp256k1::PublicKey,
        #[serde_as(as = "Hex")]
        signature: secp256k1::Signature,
    },
}

fn run(input: Input) -> Output {
    match input {
        Input::Ed25519 { path, msg } => Output::Ed25519 {
            derived_pk: ed25519::derive_public_key(&path),
            signature: ed25519::sign(path, msg),
        },
        Input::Secp256k1 { path, prehash } => Output::Secp256k1 {
            derived_pk: secp256k1::derive_public_key(&path),
            signature: secp256k1::sign(path, &prehash),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use defuse_outlayer_sdk::hex_literal::hex;

    // TODO: values

    #[test]
    fn test_ed25519() {
        assert_eq!(
            run(Input::Ed25519 {
                path: "path".to_string(),
                msg: b"message".to_vec(),
            }),
            Output::Ed25519 {
                derived_pk: hex!(
                    "00cf20e07aa9699f6c4f934230eeff8fc6f6cfdd57c8e5af93496082d75cee42"
                ),
                signature: hex!(
                    "00cf20e07aa9699f6c4f934230eeff8fc6f6cfdd57c8e5af93496082d75cee4200cf20e07aa9699f6c4f934230eeff8fc6f6cfdd57c8e5af93496082d75cee42"
                ),
            }
        );
    }

    #[test]
    fn test_secp256k1() {
        assert_eq!(
            run(Input::Secp256k1 {
                path: "path".to_string(),
                prehash: hex!("00cf20e07aa9699f6c4f934230eeff8fc6f6cfdd57c8e5af93496082d75cee42"),
            }),
            Output::Secp256k1 {
                derived_pk: hex!(
                    "00cf20e07aa9699f6c4f934230eeff8fc6f6cfdd57c8e5af93496082d75cee4200cf20e07aa9699f6c4f934230eeff8fc6f6cfdd57c8e5af93496082d75cee42"
                ),
                signature: hex!(
                    "00cf20e07aa9699f6c4f934230eeff8fc6f6cfdd57c8e5af93496082d75cee4200cf20e07aa9699f6c4f934230eeff8fc6f6cfdd57c8e5af93496082d75cee4201"
                ),
            }
        );
    }
}
