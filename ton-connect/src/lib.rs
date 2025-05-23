//! TON Connect [signData](https://docs.tonconsole.com/academy/sign-data)

use chrono::{DateTime, Utc};
use defuse_crypto::{Curve, Ed25519, Payload, SignedPayload, serde::AsCurve};
use defuse_near_utils::UnwrapOrPanicError;
use defuse_serde_utils::{base64::Base64, tlb::AsBoC};
use impl_tools::autoimpl;
use near_sdk::{env, near};
use serde_with::{PickFirst, TimestampSeconds, serde_as};
use tlb_ton::{
    Cell, Error, MsgAddress, StringError,
    r#as::{Ref, SnakeData},
    bits::ser::BitWriterExt,
};

pub use tlb_ton;

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
    /// Wallet address in either [Raw](https://docs.ton.org/learn/overviews/addresses#raw-address) representation
    /// or [user-friendly](https://docs.ton.org/learn/overviews/addresses#user-friendly-address) format
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
        match &self.payload {
            TonConnectPayloadSchema::Text { .. } | TonConnectPayloadSchema::Binary { .. } => {
                #[allow(clippy::match_wildcard_for_single_variants)]
                let (payload_prefix, payload) = match &self.payload {
                    TonConnectPayloadSchema::Text { text } => (b"txt", text.as_bytes()),
                    TonConnectPayloadSchema::Binary { bytes } => (b"bin", bytes.as_slice()),
                    _ => unreachable!(),
                };
                Ok(env::sha256_array(
                    &[
                        [0xff, 0xff].as_slice(),
                        b"ton-connect/sign-data/",
                        &self.address.workchain_id.to_be_bytes(),
                        &self.address.address,
                        &u32::try_from(self.domain.len())
                            .map_err(|_| Error::custom("domain: overflow"))?
                            .to_be_bytes(),
                        self.domain.as_bytes(),
                        &timestamp.to_be_bytes(),
                        payload_prefix,
                        &u32::try_from(payload.len())
                            .map_err(|_| Error::custom("payload: overflow"))?
                            .to_be_bytes(),
                        payload,
                    ]
                    .concat(),
                ))
            }
            TonConnectPayloadSchema::Cell { schema_crc, cell } => {
                let mut b = Cell::builder();
                b.pack(
                    #[allow(clippy::unreadable_literal)]
                    0x75569022_u32,
                )?
                .pack(schema_crc)?
                .pack(timestamp)?
                .pack(self.address)?
                .store_as::<_, Ref<SnakeData>>(&self.domain)?
                .store_as::<_, Ref>(cell)?;
                Ok(b.into_cell()
                    // use host function for recursive hash calculation
                    .hash_digest::<defuse_near_utils::digest::Sha256>())
            }
        }
    }
}

impl Payload for TonConnectPayload {
    #[inline]
    fn hash(&self) -> near_sdk::CryptoHash {
        self.try_hash().unwrap_or_panic_str()
    }
}
/// See <https://docs.tonconsole.com/academy/sign-data#choosing-the-right-format>
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
#[serde(tag = "type", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TonConnectPayloadSchema {
    Text {
        text: String,
    },
    Binary {
        #[serde_as(as = "Base64")]
        bytes: Vec<u8>,
    },
    Cell {
        schema_crc: u32,
        #[serde_as(as = "AsBoC<Base64>")]
        cell: Cell,
    },
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
    use hex_literal::hex;
    use near_sdk::serde_json;
    use rstest::rstest;
    use test_utils::random::{Seed, gen_random_bytes, make_seedable_rng, random_seed};
    use tlb_ton::UnixTimestamp;

    #[rstest]
    #[trace]
    fn verify_text(random_seed: Seed) {
        verify(
            &SignedTonConnectPayload {
                payload: TonConnectPayload {
                    address: "0:f4809e5ffac9dc42a6b1d94c5e74ad5fd86378de675c805f2274d0055cbc9378"
                        .parse()
                        .unwrap(),
                    domain: "ton-connect.github.io".to_string(),
                    timestamp: DateTime::from_timestamp(1747759882, 0).unwrap(),
                    payload: TonConnectPayloadSchema::Text {
                        text: "Hello, TON!".repeat(100),
                    },
                },
                public_key: hex!(
                    "22e795a07e832fc9084ca35a488a711f1dbedef637d4e886a6997d93ee2c2e37"
                ),
                signature: hex!(
                    "7bc628f6d634ab6ddaf10463742b13f0ede3cb828737d9ce1962cc808fbfe7035e77c1a3d0b682acf02d645cc1a244992b276552c0e1c57d30b03c2820d73d01"
                ),
            },
            random_seed,
        );
    }

    #[rstest]
    #[trace]
    fn verify_binary(random_seed: Seed) {
        verify(
            &SignedTonConnectPayload {
                payload: TonConnectPayload {
                    address: "0:f4809e5ffac9dc42a6b1d94c5e74ad5fd86378de675c805f2274d0055cbc9378"
                        .parse()
                        .unwrap(),
                    domain: "ton-connect.github.io".to_string(),
                    timestamp: DateTime::from_timestamp(1747760435, 0).unwrap(),
                    payload: TonConnectPayloadSchema::Binary {
                        bytes: hex!("48656c6c6f2c20544f4e21").into(),
                    },
                },
                public_key: hex!(
                    "22e795a07e832fc9084ca35a488a711f1dbedef637d4e886a6997d93ee2c2e37"
                ),
                signature: hex!(
                    "9cf4c1c16b47afce46940eb9cd410894f31544b74206c2254bb1651f9b32cf5b0e482b78a2e8251e54d3517fae4b06c6f23546667d63ff62dccce70451698d01"
                ),
            },
            random_seed,
        );
    }

    #[rstest]
    #[trace]
    fn verify_cell(random_seed: Seed) {
        use tlb_ton::BagOfCells;

        verify(
            &SignedTonConnectPayload {
                payload: TonConnectPayload {
                    address: "0:f4809e5ffac9dc42a6b1d94c5e74ad5fd86378de675c805f2274d0055cbc9378"
                        .parse()
                        .unwrap(),
                    domain: "ton-connect.github.io".to_string(),
                    timestamp: DateTime::from_timestamp(1747772412, 0).unwrap(),
                    payload: TonConnectPayloadSchema::Cell {
                        schema_crc: 0x2eccd0c1,
                        cell: BagOfCells::parse_base64(
                            "te6cckEBAQEAEQAAHgAAAABIZWxsbywgVE9OIb7WCx4=",
                        )
                        .unwrap()
                        .into_single_root()
                        .unwrap()
                        .as_ref()
                        .clone(),
                    },
                },
                public_key: hex!(
                    "22e795a07e832fc9084ca35a488a711f1dbedef637d4e886a6997d93ee2c2e37"
                ),
                signature: hex!(
                    "6ad083855374c201c2acb14aa4e7eef44603c8d356624c8fd3b6be3babd84bd8bc7390f0ed4484ab58a535b3088681e0006839eb07136470985b3a33bfa17c05"
                ),
            },
            random_seed,
        );
    }

    fn verify(signed: &SignedTonConnectPayload, seed: Seed) {
        verify_ok(signed, true);

        // tampering
        let mut rng = make_seedable_rng(seed);
        let bytes = gen_random_bytes(&mut rng, ..1000);
        let mut u = Unstructured::new(&bytes);
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
    #[trace]
    fn arbitrary(random_seed: Seed) {
        verify_ok(
            &Unstructured::new(&gen_random_bytes(
                &mut make_seedable_rng(random_seed),
                ..1000,
            ))
            .arbitrary()
            .unwrap(),
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
