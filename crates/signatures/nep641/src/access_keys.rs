//! Authorization via full-access keys

use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use defuse_crypto::{
    ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature},
    fmt::{ParseCurveError, TypedCurve, checked_base58_decode_array},
    secp256k1::{Secp256k1, Secp256k1RecoverableSignature, Secp256k1UncompressedPublicKey},
};
use defuse_digest::{Digest, sha3::Keccak256};
use defuse_nep413::Nep413;
use near_account_id::AccountId;

use crate::OffchainMessage;

/// Authorization via full-access key
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    // reduce collisions with other authorization schemas on offchain resolver
    serde(deny_unknown_fields),
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccessKeyAuthorization {
    /// Signed offchain message
    pub msg: OffchainMessage,

    /// Signature schema and additional metadata used during signing process
    pub via: AccessKeySchema,

    /// Access key with `FullAccess` permission
    pub access_key: PublicKey,

    /// Signature
    pub signature: Signature,
}

impl AccessKeyAuthorization {
    /// Verify the signature according to the signature schema used
    #[must_use = "check if verification passed"]
    #[inline]
    pub fn verify(&self) -> bool {
        self.via
            .verify(&self.msg, &self.access_key, &self.signature)
    }
}

/// Signature schema and additional metadata used during signing of [`AccessKeyAuthorization`].
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AccessKeySchema {
    /// [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md) signing schema.
    Nep413 {
        /// Optional callback URL, applicable to browser wallets. The URL to call after the signing
        /// process.
        #[cfg_attr(
            feature = "serde",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        callback_url: Option<String>,
    },
}

impl AccessKeySchema {
    /// Verify the signature
    #[must_use = "check if verification passed"]
    fn verify(&self, msg: &OffchainMessage, public_key: &PublicKey, signature: &Signature) -> bool {
        // only NEP-413 is supported for now
        let Self::Nep413 { callback_url } = self;

        // convert offchain message into NEP-413 payload
        let payload = msg.clone().into_nep413_payload(callback_url.clone());

        // verify
        match (public_key, signature) {
            // ed25519
            (PublicKey::Ed25519(pk), Signature::Ed25519(sig)) => {
                let Ok(pk) = pk.try_into() else {
                    return false;
                };
                Nep413::verify::<Ed25519>(&pk, &payload, &sig.into())
            }

            // secp256k1
            (PublicKey::Secp256k1(pk), Signature::Secp256k1(sig)) => {
                let Ok(pk) = pk.try_into() else {
                    return false;
                };
                let Ok(sig) = sig.try_into() else {
                    return false;
                };
                Nep413::verify::<Secp256k1>(&pk, &payload, &sig)
            }

            // curve mismatch
            _ => false,
        }
    }
}

/// Public key for [`AccessKeyAuthorization`]
#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr)
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema)),
    borsh(use_discriminant = true)
)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From)]
#[non_exhaustive]
#[repr(u8)]
pub enum PublicKey {
    Ed25519(Ed25519PublicKey) = 0,
    Secp256k1(Secp256k1UncompressedPublicKey) = 1,
    // TODO: MlDsa65 (full, not hashed) = 2,
}

impl PublicKey {
    /// Derive implicit account ID from this public key
    #[inline]
    pub fn to_implicit_account_id(&self) -> AccountId {
        match self {
            Self::Ed25519(pk) => {
                // https://docs.near.org/concepts/protocol/account-id#implicit-address
                hex::encode(pk)
            }
            Self::Secp256k1(pk) => {
                // https://ethereum.org/en/developers/docs/accounts/#account-creation
                format!("0x{}", hex::encode(&Keccak256::digest(pk)[12..32]))
            }
        }
        .try_into()
        .unwrap_or_else(|_| unreachable!())
    }
}

impl Debug for PublicKey {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ed25519(pk) => pk.to_string(),
                Self::Secp256k1(pk) => pk.to_string(),
            }
        )
    }
}

impl Display for PublicKey {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl FromStr for PublicKey {
    type Err = ParseCurveError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (curve, data) = s.split_once(':').ok_or(ParseCurveError::WrongCurveType)?;

        match curve {
            Ed25519::CURVE_TYPE => checked_base58_decode_array(data)
                .map(Ed25519PublicKey)
                .map(Into::into),
            Secp256k1::CURVE_TYPE => checked_base58_decode_array(data)
                .map(Secp256k1UncompressedPublicKey)
                .map(Into::into),
            _ => Err(ParseCurveError::WrongCurveType),
        }
    }
}

#[cfg(feature = "near-kit")]
impl From<PublicKey> for ::near_kit::PublicKey {
    #[inline]
    fn from(pk: PublicKey) -> Self {
        match pk {
            PublicKey::Ed25519(pk) => Self::Ed25519(pk.0),
            PublicKey::Secp256k1(pk) => Self::Secp256k1(pk.0),
        }
    }
}

/// Signature for [`AccessKeyAuthorization`]
#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr)
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema)),
    borsh(use_discriminant = true)
)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From)]
#[non_exhaustive]
#[repr(u8)]
pub enum Signature {
    Ed25519(Ed25519Signature) = 0,
    Secp256k1(Secp256k1RecoverableSignature) = 1,
}

impl Debug for Signature {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ed25519(sig) => sig.to_string(),
                Self::Secp256k1(sig) => sig.to_string(),
            }
        )
    }
}

impl Display for Signature {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl FromStr for Signature {
    type Err = ParseCurveError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (curve, data) = s.split_once(':').ok_or(ParseCurveError::WrongCurveType)?;

        match curve {
            Ed25519::CURVE_TYPE => checked_base58_decode_array(data)
                .map(Ed25519Signature)
                .map(Into::into),
            Secp256k1::CURVE_TYPE => checked_base58_decode_array(data)
                .map(Secp256k1RecoverableSignature)
                .map(Into::into),
            _ => Err(ParseCurveError::WrongCurveType),
        }
    }
}

#[cfg(feature = "schemars-v0_8")]
const _: () = {
    use std::borrow::Cow;

    use schemars::{
        JsonSchema, SchemaGenerator,
        schema::{InstanceType, Metadata, Schema, SchemaObject},
    };

    impl JsonSchema for PublicKey {
        #[inline]
        fn schema_name() -> String {
            "PublicKey".to_owned()
        }

        #[inline]
        fn schema_id() -> Cow<'static, str> {
            Cow::Borrowed(concat!(module_path!(), "::", "PublicKey"))
        }

        #[inline]
        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                metadata: Some(
                    Metadata {
                        examples: [Self::example_ed25519(), Self::example_secp256k1()]
                            .map(serde_json::to_value)
                            .map(Result::unwrap)
                            .into(),
                        ..Default::default()
                    }
                    .into(),
                ),
                ..Default::default()
            }
            .into()
        }
    }

    impl PublicKey {
        #[inline]
        fn example_ed25519() -> Self {
            "ed25519:5TagutioHgKLh7KZ1VEFBYfgRkPtqnKm9LoMnJMJugxm"
                .parse()
                .unwrap()
        }

        #[inline]
        fn example_secp256k1() -> Self {
            "secp256k1:3aMVMxsoAnHUbweXMtdKaN1uJaNwsfKv7wnc97SDGjXhyK62VyJwhPUPLZefKVthcoUcuWK6cqkSU4M542ipNxS3"
                .parse()
                .unwrap()
        }
    }

    impl JsonSchema for Signature {
        #[inline]
        fn schema_name() -> String {
            "Signature".to_owned()
        }

        #[inline]
        fn schema_id() -> Cow<'static, str> {
            Cow::Borrowed(concat!(module_path!(), "::", "Signature"))
        }

        #[inline]
        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                metadata: Some(
                    Metadata {
                        examples: [Self::example_ed25519(), Self::example_secp256k1()]
                            .map(serde_json::to_value)
                            .map(Result::unwrap)
                            .into(),
                        ..Default::default()
                    }
                    .into(),
                ),
                ..Default::default()
            }
            .into()
        }
    }

    impl Signature {
        #[inline]
        fn example_ed25519() -> Self {
            "ed25519:DNxoVu7L7sHr9pcHGWQoJtPsrwheB8akht1JxaGpc9hGrpehdycXBMLJg4ph1bQ9bXdfoxJCbbwxj3Bdrda52eF"
                .parse()
                .unwrap()
        }

        #[inline]
        fn example_secp256k1() -> Self {
            "secp256k1:7huDZxNnibusy6wFkbUBQ9Rqq2VmCKgTWYdJwcPj8VnciHjZKPa41rn5n6WZnMqSUCGRHWMAsMjKGtMVVmpETCeCs"
                .parse()
                .unwrap()
        }
    }
};
