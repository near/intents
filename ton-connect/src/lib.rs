//! TON Connect [signData](https://docs.tonconsole.com/academy/sign-data)

use defuse_crypto::{Curve, Ed25519, Payload, SignedPayload, serde::AsCurve};
use defuse_near_utils::UnwrapOrPanicError;
use defuse_serde_utils::base64::Base64;
use impl_tools::autoimpl;
use near_sdk::{env, near};
use serde_with::serde_as;
use tlb::{
    Cell,
    r#as::Text,
    ser::{CellBuilder, CellBuilderError, CellSerialize, CellSerializeExt},
};
use tlb_ton::MsgAddress;

#[near(serializers = [json])]
#[autoimpl(Deref using self.payload)]
#[derive(Debug, Clone)]
pub struct Tep104Payload {
    pub address: MsgAddress,
    pub domain: String,
    pub timestamp: u64, // TODO: chrono?
    pub payload: Tep104SchemaPayload,
}

impl Tep104Payload {
    fn prehash(&self) -> Vec<u8> {
        match &self.payload {
            Tep104SchemaPayload::Text { .. } | Tep104SchemaPayload::Binary { .. } => {
                let (payload_prefix, payload) = match &self.payload {
                    Tep104SchemaPayload::Text { text } => (b"txt", text.as_bytes()),
                    Tep104SchemaPayload::Binary { bytes } => (b"bin", bytes.as_slice()),
                    _ => unreachable!(),
                };
                [
                    [0xff, 0xff].as_slice(),
                    b"ton-connect/sign-data/",
                    &self.address.workchain_id.to_be_bytes(),
                    &self.address.address,
                    &u32::try_from(self.domain.len())
                        .map_err(|_| "domain: too long")
                        .unwrap_or_panic_static_str()
                        .to_be_bytes(),
                    self.domain.as_bytes(),
                    &self.timestamp.to_be_bytes(),
                    payload_prefix,
                    &u32::try_from(payload.len())
                        .map_err(|_| "payload: too long")
                        .unwrap_or_panic_static_str()
                        .to_be_bytes(),
                    payload,
                ]
                .concat()
            }
            Tep104SchemaPayload::Cell { schema_crc } => todo!(),
        }
    }
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[serde(tag = "type", rename_all = "snake_case")]
#[derive(Debug, Clone)]
pub enum Tep104SchemaPayload {
    // TODO: docs
    // This schema is used to sign UTF-8 text messages using
    // _snake format_ (per [TEP-64](https://github.com/ton-blockchain/TEPs/blob/master/text/0064-token-data-standard.md)).
    // ```
    // crc32('plaintext text:Text = PayloadCell') = 0x754bf91b
    // ```
    Text {
        text: String,
    },
    Binary {
        #[serde_as(as = "Base64")]
        bytes: Vec<u8>,
    },
    Cell {
        schema_crc: u32,
        // TODO
        // cell: Cell,
    },
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[autoimpl(Deref using self.payload)]
#[derive(Debug, Clone)]
pub struct SignedTep104Payload {
    #[serde(flatten)]
    pub payload: Tep104Payload,

    #[serde_as(as = "AsCurve<Ed25519>")]
    pub public_key: <Ed25519 as Curve>::PublicKey,
    #[serde_as(as = "AsCurve<Ed25519>")]
    pub signature: <Ed25519 as Curve>::Signature,
}

impl Payload for SignedTep104Payload {
    #[inline]
    fn hash(&self) -> near_sdk::CryptoHash {
        env::sha256_array(&self.payload.prehash())
    }
}

impl SignedPayload for SignedTep104Payload {
    type PublicKey = <Ed25519 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Ed25519::verify(&self.signature, &self.hash(), &self.public_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use hex_literal::hex;

    #[test]
    fn verify_text() {
        let signed = SignedTep104Payload {
            payload: Tep104Payload {
                address: "0:f4809e5ffac9dc42a6b1d94c5e74ad5fd86378de675c805f2274d0055cbc9378"
                    .parse()
                    .unwrap(),
                domain: "ton-connect.github.io".to_string(),
                timestamp: 1747759882,
                payload: Tep104SchemaPayload::Text {
                    text: "Hello, TON!".repeat(100),
                },
            },
            public_key: hex!("22e795a07e832fc9084ca35a488a711f1dbedef637d4e886a6997d93ee2c2e37"),
            signature: hex!(
                "7bc628f6d634ab6ddaf10463742b13f0ede3cb828737d9ce1962cc808fbfe7035e77c1a3d0b682acf02d645cc1a244992b276552c0e1c57d30b03c2820d73d01"
            ),
        };

        assert_eq!(signed.verify(), Some(signed.public_key));
    }

    #[test]
    fn verify_binary() {
        let signed = SignedTep104Payload {
            payload: Tep104Payload {
                address: "0:f4809e5ffac9dc42a6b1d94c5e74ad5fd86378de675c805f2274d0055cbc9378"
                    .parse()
                    .unwrap(),
                domain: "ton-connect.github.io".to_string(),
                timestamp: 1747760435,
                payload: Tep104SchemaPayload::Binary {
                    bytes: hex!("48656c6c6f2c20544f4e21").into(),
                },
            },
            public_key: hex!("22e795a07e832fc9084ca35a488a711f1dbedef637d4e886a6997d93ee2c2e37"),
            signature: hex!(
                "9cf4c1c16b47afce46940eb9cd410894f31544b74206c2254bb1651f9b32cf5b0e482b78a2e8251e54d3517fae4b06c6f23546667d63ff62dccce70451698d01"
            ),
        };

        assert_eq!(signed.verify(), Some(signed.public_key));
    }

    #[test]
    fn verify_cell() {
        let signed = SignedTep104Payload {
            payload: Tep104Payload {
                address: "0:f4809e5ffac9dc42a6b1d94c5e74ad5fd86378de675c805f2274d0055cbc9378"
                    .parse()
                    .unwrap(),
                domain: "ton-connect.github.io".to_string(),
                timestamp: 1747760435,
                payload: Tep104SchemaPayload::Cell { schema_crc: 1 },
            },
            public_key: hex!("22e795a07e832fc9084ca35a488a711f1dbedef637d4e886a6997d93ee2c2e37"),
            signature: hex!(
                "9cf4c1c16b47afce46940eb9cd410894f31544b74206c2254bb1651f9b32cf5b0e482b78a2e8251e54d3517fae4b06c6f23546667d63ff62dccce70451698d01"
            ),
        };

        assert_eq!(signed.verify(), Some(signed.public_key));
    }
}
