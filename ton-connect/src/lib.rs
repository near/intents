//! TON Connect [signData](https://github.com/ton-blockchain/ton-connect/blob/main/requests-responses.md#sign-data)
mod schema;

use chrono::{DateTime, Utc};
use defuse_crypto::{Curve, Ed25519, Payload, SignedPayload, serde::AsCurve};
use defuse_near_utils::UnwrapOrPanicError;
use impl_tools::autoimpl;
use near_sdk::near;
use serde_with::{PickFirst, TimestampSeconds, serde_as};
use tlb_ton::{Error, MsgAddress, StringError};

pub use schema::TonConnectPayloadSchema;
pub use tlb_ton;

use crate::schema::{PayloadSchema, TonConnectPayloadContext};

#[cfg_attr(test, derive(arbitrary::Arbitrary))]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TonConnectPayload {
    /// Wallet address in either [Raw](https://docs.ton.org/v3/documentation/smart-contracts/addresses/address-formats#raw-address) representation
    /// or [user-friendly](https://docs.ton.org/v3/documentation/smart-contracts/addresses/address-formats#user-friendly-address) format
    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        schemars(with = "String")
    )]
    pub address: MsgAddress,
    /// dApp domain
    pub domain: String,
    /// UNIX timestamp (in seconds or RFC3339) at the time of singing
    #[cfg_attr(test, arbitrary(with = ::tlb_ton::UnixTimestamp::arbitrary))]
    #[serde_as(as = "PickFirst<(_, TimestampSeconds)>")]
    pub timestamp: DateTime<Utc>,
    pub payload: TonConnectPayloadSchema,
}

impl TonConnectPayload {
    fn try_hash(&self) -> Result<near_sdk::CryptoHash, StringError> {
        let timestamp: u64 = self
            .timestamp
            .timestamp()
            .try_into()
            .map_err(|_| Error::custom("negative timestamp"))?;

        let context = TonConnectPayloadContext {
            address: self.address,
            domain: self.domain.clone(),
            timestamp,
        };

        self.payload.hash_with_context(context)
    }
}

impl Payload for TonConnectPayload {
    #[inline]
    fn hash(&self) -> near_sdk::CryptoHash {
        self.try_hash().unwrap_or_panic_str()
    }
}

#[cfg_attr(test, derive(arbitrary::Arbitrary))]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedTonConnectPayload {
    #[serde(flatten)]
    pub payload: TonConnectPayload,

    #[serde_as(as = "AsCurve<Ed25519>")]
    pub public_key: <Ed25519 as Curve>::PublicKey,
    #[serde_as(as = "AsCurve<Ed25519>")]
    pub signature: <Ed25519 as Curve>::Signature,
}

impl Payload for SignedTonConnectPayload {
    #[inline]
    fn hash(&self) -> near_sdk::CryptoHash {
        self.payload.hash()
    }
}

impl SignedPayload for SignedTonConnectPayload {
    type PublicKey = <Ed25519 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Ed25519::verify(&self.signature, &self.hash(), &self.public_key)
    }
}

#[cfg(test)]
#[allow(clippy::unreadable_literal)]
mod tests {
    use super::*;

    use arbitrary::{Arbitrary, Unstructured};
    use defuse_test_utils::random::random_bytes;
    use hex_literal::hex;
    use near_sdk::serde_json;
    use rstest::rstest;
    use tlb_ton::UnixTimestamp;

    #[cfg(feature = "text")]
    #[rstest]
    fn verify_text(random_bytes: Vec<u8>) {
        verify(
            &SignedTonConnectPayload {
                payload: TonConnectPayload {
                    address: "0:f4809e5ffac9dc42a6b1d94c5e74ad5fd86378de675c805f2274d0055cbc9378"
                        .parse()
                        .unwrap(),
                    domain: "ton-connect.github.io".to_string(),
                    timestamp: DateTime::from_timestamp(1747759882, 0).unwrap(),
                    payload: TonConnectPayloadSchema::text(&"Hello, TON!".repeat(100)),
                },
                public_key: hex!(
                    "22e795a07e832fc9084ca35a488a711f1dbedef637d4e886a6997d93ee2c2e37"
                ),
                signature: hex!(
                    "7bc628f6d634ab6ddaf10463742b13f0ede3cb828737d9ce1962cc808fbfe7035e77c1a3d0b682acf02d645cc1a244992b276552c0e1c57d30b03c2820d73d01"
                ),
            },
            &random_bytes,
        );
    }

    #[cfg(feature = "binary")]
    #[rstest]
    fn verify_binary(random_bytes: Vec<u8>) {
        verify(
            &SignedTonConnectPayload {
                payload: TonConnectPayload {
                    address: "0:f4809e5ffac9dc42a6b1d94c5e74ad5fd86378de675c805f2274d0055cbc9378"
                        .parse()
                        .unwrap(),
                    domain: "ton-connect.github.io".to_string(),
                    timestamp: DateTime::from_timestamp(1747760435, 0).unwrap(),
                    payload: TonConnectPayloadSchema::binary(&hex!("48656c6c6f2c20544f4e21")),
                },
                public_key: hex!(
                    "22e795a07e832fc9084ca35a488a711f1dbedef637d4e886a6997d93ee2c2e37"
                ),
                signature: hex!(
                    "9cf4c1c16b47afce46940eb9cd410894f31544b74206c2254bb1651f9b32cf5b0e482b78a2e8251e54d3517fae4b06c6f23546667d63ff62dccce70451698d01"
                ),
            },
            &random_bytes,
        );
    }

    #[cfg(feature = "cell")]
    #[rstest]
    fn verify_cell(random_bytes: Vec<u8>) {
        use tlb_ton::BagOfCells;

        verify(
            &SignedTonConnectPayload {
                payload: TonConnectPayload {
                    address: "0:f4809e5ffac9dc42a6b1d94c5e74ad5fd86378de675c805f2274d0055cbc9378"
                        .parse()
                        .unwrap(),
                    domain: "ton-connect.github.io".to_string(),
                    timestamp: DateTime::from_timestamp(1747772412, 0).unwrap(),
                    payload: TonConnectPayloadSchema::cell(
                        0x2eccd0c1,
                        BagOfCells::parse_base64("te6cckEBAQEAEQAAHgAAAABIZWxsbywgVE9OIb7WCx4=")
                            .unwrap()
                            .into_single_root()
                            .unwrap()
                            .as_ref()
                            .clone(),
                    ),
                },
                public_key: hex!(
                    "22e795a07e832fc9084ca35a488a711f1dbedef637d4e886a6997d93ee2c2e37"
                ),
                signature: hex!(
                    "6ad083855374c201c2acb14aa4e7eef44603c8d356624c8fd3b6be3babd84bd8bc7390f0ed4484ab58a535b3088681e0006839eb07136470985b3a33bfa17c05"
                ),
            },
            &random_bytes,
        );
    }

    fn verify(signed: &SignedTonConnectPayload, random_bytes: &[u8]) {
        verify_ok(signed, true);

        // tampering
        let mut u = Unstructured::new(random_bytes);
        {
            let mut t = signed.clone();
            t.payload.address = Arbitrary::arbitrary(&mut u).unwrap();
            dbg!(&t.payload.address);
            verify_ok(&t, false);
        }
        {
            let mut t = signed.clone();
            t.payload.domain = Arbitrary::arbitrary(&mut u).unwrap();
            dbg!(&t.payload.domain);
            verify_ok(&t, false);
        }
        {
            let mut t = signed.clone();
            t.payload.timestamp = UnixTimestamp::arbitrary(&mut u).unwrap();
            dbg!(&t.payload.timestamp);
            verify_ok(&t, false);
        }
        {
            let mut t = signed.clone();
            t.payload.payload = Arbitrary::arbitrary(&mut u).unwrap();
            dbg!(&t.payload.payload);
            verify_ok(&t, false);
        }
    }

    #[rstest]
    fn arbitrary(random_bytes: Vec<u8>) {
        verify_ok(
            &Unstructured::new(&random_bytes).arbitrary().unwrap(),
            false,
        );
    }

    fn verify_ok(signed: &SignedTonConnectPayload, ok: bool) {
        let serialized = serde_json::to_string_pretty(signed).unwrap();
        println!("{}", &serialized);
        let deserialized: SignedTonConnectPayload = serde_json::from_str(&serialized).unwrap();

        assert_eq!(&deserialized, signed);
        assert_eq!(deserialized.verify(), ok.then_some(deserialized.public_key));
    }
}
