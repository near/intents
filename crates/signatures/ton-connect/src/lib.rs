//! TON Connect [signData](https://docs.tonconsole.com/academy/sign-data)
#[cfg(feature = "cell")]
mod cell;

use defuse_crypto::{Curve, ed25519::Ed25519};
use defuse_digest::{Digest, sha2::Sha256};
use defuse_time::Timestamp;
#[cfg(feature = "arbitrary")]
use defuse_time::arbitrary::RangeNanos;
#[cfg(feature = "cell")]
pub use tlb_ton::Cell;
pub use tlb_ton::MsgAddress;

/// [TON Connect](https://docs.tonconsole.com/academy/sign-data) signature schema.
pub struct TonConnect;

impl TonConnect {
    #[must_use = "check if verification passed"]
    #[inline]
    pub fn verify(
        public_key: &<Ed25519 as Curve>::PublicKey,
        payload: &TonConnectPayload,
        signature: &<Ed25519 as Curve>::Signature,
    ) -> bool {
        let Some(prehash) = payload.try_prehash() else {
            return false;
        };

        Ed25519::verify(public_key, &prehash, signature)
    }
}

/// [TON Connect](https://docs.tonconsole.com/academy/sign-data) signable payload.
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TonConnectPayload {
    /// Wallet address in either [Raw](https://docs.ton.org/v3/documentation/smart-contracts/addresses/address-formats#raw-address) representation
    /// or [user-friendly](https://docs.ton.org/v3/documentation/smart-contracts/addresses/address-formats#user-friendly-address) format
    pub address: MsgAddress,

    /// dApp domain
    pub domain: String,

    /// UNIX timestamp (RFC3339 or in seconds) at the time of singing
    #[cfg_attr(
        feature = "arbitrary",
        arbitrary(with = ::arbitrary_with::As::<RangeNanos::<0>>::arbitrary)
    )]
    #[cfg_attr(
        feature = "serde",
        serde_as(as = "::serde_with::PickFirst<(
            _,
            ::defuse_time::serde::TimestampSeconds<::serde_with::DisplayFromStr>,
            ::defuse_time::serde::TimestampSeconds,
        )>")
    )]
    pub timestamp: Timestamp,

    /// Typed payload schema
    pub payload: TonConnectPayloadSchema,
}

impl TonConnectPayload {
    pub fn try_prehash(&self) -> Option<[u8; 32]> {
        let timestamp: u64 = self.timestamp.as_secs().try_into().ok()?;

        let (prefix, payload) = match &self.payload {
            TonConnectPayloadSchema::Text { text } => (b"txt", text.as_bytes()),
            TonConnectPayloadSchema::Binary { bytes } => (b"bin", bytes.as_slice()),
            #[cfg(feature = "cell")]
            TonConnectPayloadSchema::Cell { schema_crc, cell } => {
                return self::cell::TonConnectCellMessage {
                    schema_crc: *schema_crc,
                    timestamp,
                    user_address: &self.address,
                    app_domain: &self.domain,
                    payload: &cell,
                }
                .hash();
            }
        };

        let domain_len: u32 = self.domain.len().try_into().ok()?;
        let payload_len: u32 = payload.len().try_into().ok()?;

        // 0xffff ++ "ton-connect/sign-data/" ++ Address ++ AppDomain ++ Timestamp ++ Payload
        let prehash = Sha256::new_with_prefix(b"\xFF\xFFton-connect/sign-data/")
            .chain_update(self.address.workchain_id.to_be_bytes())
            .chain_update(self.address.address)
            .chain_update(domain_len.to_be_bytes())
            .chain_update(self.domain.as_bytes())
            .chain_update(timestamp.to_be_bytes())
            .chain_update(prefix)
            .chain_update(payload_len.to_be_bytes())
            .chain_update(payload)
            .finalize()
            .into();

        Some(prehash)
    }
}

/// [`TonConnectPayload`](TonConnectPayload) schema.
///
/// See <https://docs.tonconsole.com/academy/sign-data#choosing-the-right-format>
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    serde(tag = "type", rename_all = "snake_case")
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TonConnectPayloadSchema {
    /// Text payload. Use this when the data is human-readable.
    ///
    /// See <https://docs.tonconsole.com/academy/sign-data#1-text>
    Text { text: String },

    /// Binary payload. Use this when signing a hash, arbitrary bytes,
    /// or a file.
    ///
    /// See <https://docs.tonconsole.com/academy/sign-data#2-binary>
    Binary {
        #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::base64::Base64"))]
        bytes: Vec<u8>,
    },

    /// Cell payload. Use this if the signed data should be verifiable and
    /// restorable inside a smart contract.
    ///
    /// See <https://docs.tonconsole.com/academy/sign-data#3-cell>
    #[cfg(feature = "cell")]
    Cell {
        /// Schema CRC: `crc32(schema)`
        schema_crc: u32,

        /// Data serialized to [`Cell`] according to schema
        #[cfg_attr(
            feature = "serde",
            serde_as(as = "defuse_serde_utils::tlb::AsBoC<serde_with::base64::Base64>")
        )]
        cell: Cell,
    },
}

impl TonConnectPayloadSchema {
    pub fn text(txt: impl Into<String>) -> Self {
        Self::Text { text: txt.into() }
    }

    pub fn binary(bytes: impl Into<Vec<u8>>) -> Self {
        Self::Binary {
            bytes: bytes.into(),
        }
    }

    #[cfg(feature = "cell")]
    pub const fn cell(schema_crc: u32, cell: Cell) -> Self {
        Self::Cell { schema_crc, cell }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use defuse_crypto::ed25519::ed25519_dalek::{
        PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH, Signature, VerifyingKey,
    };
    use hex_literal::hex;
    use rstest::rstest;
    use tlb_ton::BagOfCells;

    #[rstest]
    #[case::text(
        hex!("22e795a07e832fc9084ca35a488a711f1dbedef637d4e886a6997d93ee2c2e37"),
        TonConnectPayload {
            address: "0:f4809e5ffac9dc42a6b1d94c5e74ad5fd86378de675c805f2274d0055cbc9378"
                .parse()
                .unwrap(),
            domain: "ton-connect.github.io".to_string(),
            timestamp: Timestamp::from_secs(1747759882).unwrap(),
            payload: TonConnectPayloadSchema::text("Hello, TON!".repeat(100)),
        },
        hex!("7bc628f6d634ab6ddaf10463742b13f0ede3cb828737d9ce1962cc808fbfe7035e77c1a3d0b682acf02d645cc1a244992b276552c0e1c57d30b03c2820d73d01"),
    )]
    #[case::binary(
        hex!("22e795a07e832fc9084ca35a488a711f1dbedef637d4e886a6997d93ee2c2e37"),
        TonConnectPayload {
            address: "0:f4809e5ffac9dc42a6b1d94c5e74ad5fd86378de675c805f2274d0055cbc9378"
                .parse()
                .unwrap(),
            domain: "ton-connect.github.io".to_string(),
            timestamp: Timestamp::from_secs(1747760435).unwrap(),
            payload: TonConnectPayloadSchema::binary(hex!("48656c6c6f2c20544f4e21")),
        },
        hex!("9cf4c1c16b47afce46940eb9cd410894f31544b74206c2254bb1651f9b32cf5b0e482b78a2e8251e54d3517fae4b06c6f23546667d63ff62dccce70451698d01"),
    )]
    #[cfg_attr(feature = "cell", case::cell(
        hex!("22e795a07e832fc9084ca35a488a711f1dbedef637d4e886a6997d93ee2c2e37"),
        TonConnectPayload {
            address: "0:f4809e5ffac9dc42a6b1d94c5e74ad5fd86378de675c805f2274d0055cbc9378"
                .parse()
                .unwrap(),
            domain: "ton-connect.github.io".to_string(),
            timestamp: Timestamp::from_secs(1747772412).unwrap(),
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
        hex!("6ad083855374c201c2acb14aa4e7eef44603c8d356624c8fd3b6be3babd84bd8bc7390f0ed4484ab58a535b3088681e0006839eb07136470985b3a33bfa17c05"),
    ))]
    fn verify_ok(
        #[case] public_key: [u8; PUBLIC_KEY_LENGTH],
        #[case] payload: TonConnectPayload,
        #[case] signature: [u8; SIGNATURE_LENGTH],
    ) {
        let public_key = VerifyingKey::from_bytes(&public_key).unwrap();
        let signature = Signature::from_bytes(&signature);

        assert!(
            TonConnect::verify(&public_key, &payload, &signature),
            "invalid signature"
        );
    }
}
