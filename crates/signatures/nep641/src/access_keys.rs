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

#[cfg(feature = "json")]
const _: () = {
    impl From<&AccessKeyAuthorization> for String {
        /// Convert to the authorization blob
        #[inline]
        fn from(auth: &AccessKeyAuthorization) -> Self {
            serde_json::to_string(auth).expect("JSON: failed to serialize")
        }
    }

    impl From<AccessKeyAuthorization> for String {
        /// Convert to the authorization blob
        #[inline]
        fn from(auth: AccessKeyAuthorization) -> Self {
            (&auth).into()
        }
    }
};

/// Signature schema and additional metadata used during signing of [`AccessKeyAuthorization`].
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    serde(tag = "schema", content = "extra", rename_all = "snake_case")
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
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

#[cfg(feature = "near-kit")]
const _: () = {
    impl PublicKey {
        #[allow(clippy::needless_pass_by_value)]
        #[must_use]
        #[inline]
        pub fn from_kit(pk: near_kit::PublicKey) -> Option<Self> {
            match pk {
                near_kit::PublicKey::Ed25519(pk) => Some(Self::Ed25519(pk.into())),
                near_kit::PublicKey::Secp256k1(pk) => Some(Self::Secp256k1(pk.into())),
                _ => None,
            }
        }
    }

    impl From<PublicKey> for ::near_kit::PublicKey {
        #[inline]
        fn from(pk: PublicKey) -> Self {
            match pk {
                PublicKey::Ed25519(pk) => Self::Ed25519(pk.0),
                PublicKey::Secp256k1(pk) => Self::Secp256k1(pk.0),
            }
        }
    }

    impl Signature {
        #[allow(clippy::needless_pass_by_value)]
        #[must_use]
        #[inline]
        pub fn from_kit(sig: near_kit::Signature) -> Option<Self> {
            #[allow(clippy::match_wildcard_for_single_variants)]
            match sig {
                near_kit::Signature::Ed25519(sig) => Some(Self::Ed25519(sig.into())),
                near_kit::Signature::Secp256k1(sig) => Some(Self::Secp256k1(sig.into())),
                _ => None,
            }
        }
    }

    impl From<Signature> for near_kit::Signature {
        #[inline]
        fn from(sig: Signature) -> Self {
            match sig {
                Signature::Ed25519(sig) => Self::Ed25519(sig.0),
                Signature::Secp256k1(sig) => Self::Secp256k1(sig.0),
            }
        }
    }
};

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

#[cfg(all(test, feature = "json"))]
mod tests {
    use super::*;

    /// Canonical test vectors, published for implementations in other languages.
    const VECTORS: &str = include_str!("../vectors/access_key_authorization.json");

    #[derive(::serde::Deserialize)]
    struct TestVectors {
        nep413_prefix_tag: u32,
        nep413_payloads: Vec<PayloadVector>,
        signed: Vec<SignedVector>,
        verify_cases: Vec<VerifyCase>,
        parse_cases: Vec<ParseCase>,
        account_id_cases: AccountIdCases,
        implicit_accounts: Vec<ImplicitAccount>,
    }

    #[derive(::serde::Deserialize)]
    struct PayloadVector {
        name: String,
        message: OffchainMessage,
        callback_url: Option<String>,
        recipient: String,
        nonce: String,
        payload_json: String,
        prehash_preimage: String,
        prehash: String,
        offchain_message_hash: String,
    }

    #[derive(::serde::Deserialize)]
    struct SignedVector {
        name: String,
        curve: String,
        message: OffchainMessage,
        callback_url: Option<String>,
        access_key: String,
        signature: String,
        prehash: String,
        implicit_account_id: String,
        authorization: String,
        verifies: bool,
    }

    #[derive(::serde::Deserialize)]
    struct VerifyCase {
        name: String,
        authorization: String,
        verifies: bool,
    }

    #[derive(::serde::Deserialize)]
    struct ParseCase {
        name: String,
        blob: String,
        parses: bool,
    }

    #[derive(::serde::Deserialize)]
    struct AccountIdCases {
        template: String,
        cases: Vec<AccountIdCase>,
    }

    #[derive(::serde::Deserialize)]
    struct AccountIdCase {
        account_id: String,
        field: String,
        parses: bool,
    }

    #[derive(::serde::Deserialize)]
    struct ImplicitAccount {
        name: String,
        curve: String,
        public_key: String,
        #[serde(rename = "implicit_account_id")]
        account_id: String,
    }

    fn vectors() -> TestVectors {
        serde_json::from_str(VECTORS).expect("invalid test vectors")
    }

    fn assert_curve(name: &str, curve: &str, public_key: &PublicKey) {
        match (curve, public_key) {
            ("ed25519", PublicKey::Ed25519(_)) | ("secp256k1", PublicKey::Secp256k1(_)) => {}
            _ => panic!("vector '{name}': curve is not {curve}"),
        }
    }

    /// [`OffchainMessage::into_nep413_payload`] and the NEP-413 prehash it is signed under.
    #[test]
    fn nep413_payload_vectors() {
        let TestVectors {
            nep413_prefix_tag,
            nep413_payloads,
            ..
        } = vectors();
        assert!(!nep413_payloads.is_empty(), "no test vectors");

        for PayloadVector {
            name,
            message,
            callback_url,
            recipient,
            nonce,
            payload_json,
            prehash_preimage,
            prehash,
            offchain_message_hash,
        } in nep413_payloads
        {
            let payload = message.clone().into_nep413_payload(callback_url);

            assert_eq!(payload.recipient, recipient, "vector '{name}'");
            assert_eq!(payload.message, message.payload, "vector '{name}'");
            assert_eq!(
                serde_json::to_string(&payload).expect("JSON"),
                payload_json,
                "vector '{name}'",
            );

            // the prehash preimage is the prefix tag followed by the borsh-encoded payload
            let preimage = hex::decode(&prehash_preimage).expect("hex");
            let (tag, encoded) = preimage.split_at(size_of::<u32>());
            assert_eq!(tag, nep413_prefix_tag.to_le_bytes(), "vector '{name}'");
            assert_eq!(
                encoded,
                ::borsh::to_vec(&payload).expect("borsh"),
                "vector '{name}'",
            );
            assert_eq!(
                hex::encode(Nep413::prehash(&payload)),
                prehash,
                "vector '{name}'"
            );

            // the nonce binds the whole message via its canonical hash, which is a
            // *different* hash function over a different preimage than the prehash
            assert_eq!(hex::encode(payload.nonce), nonce, "vector '{name}'");
            assert_eq!(
                hex::encode(message.hash()),
                offchain_message_hash,
                "vector '{name}'"
            );
            assert_eq!(nonce, offchain_message_hash, "vector '{name}'");
            assert_ne!(prehash, offchain_message_hash, "vector '{name}'");
        }
    }

    /// Complete authorization blobs that MUST verify.
    #[test]
    fn signed_vectors() {
        let TestVectors { signed, .. } = vectors();
        assert!(!signed.is_empty(), "no test vectors");

        for SignedVector {
            name,
            curve,
            message,
            callback_url,
            access_key,
            signature,
            prehash,
            implicit_account_id,
            authorization,
            verifies,
        } in signed
        {
            let auth: AccessKeyAuthorization =
                serde_json::from_str(&authorization).unwrap_or_else(|_| panic!("vector '{name}'"));

            assert_eq!(auth.msg, message, "vector '{name}'");
            assert_eq!(
                auth.via,
                AccessKeySchema::Nep413 {
                    callback_url: callback_url.clone()
                },
                "vector '{name}'",
            );
            assert_eq!(auth.access_key.to_string(), access_key, "vector '{name}'");
            assert_eq!(auth.signature.to_string(), signature, "vector '{name}'");
            assert_curve(&name, &curve, &auth.access_key);

            // the blob round-trips byte-for-byte
            assert_eq!(String::from(&auth), authorization, "vector '{name}'");

            assert_eq!(
                hex::encode(Nep413::prehash(&message.into_nep413_payload(callback_url))),
                prehash,
                "vector '{name}'",
            );
            assert_eq!(
                auth.access_key.to_implicit_account_id().as_str(),
                implicit_account_id,
                "vector '{name}'",
            );

            assert!(verifies, "vector '{name}': must be a positive case");
            assert_eq!(auth.verify(), verifies, "vector '{name}'");
        }
    }

    /// Blobs that parse, but MUST NOT verify.
    #[test]
    fn verify_case_vectors() {
        let TestVectors { verify_cases, .. } = vectors();
        assert!(!verify_cases.is_empty(), "no test vectors");

        for VerifyCase {
            name,
            authorization,
            verifies,
        } in verify_cases
        {
            let auth: AccessKeyAuthorization = serde_json::from_str(&authorization)
                .unwrap_or_else(|_| panic!("vector '{name}': must parse"));

            assert!(!verifies, "vector '{name}': must be a negative case");
            assert_eq!(auth.verify(), verifies, "vector '{name}'");
        }
    }

    /// The JSON accept/reject boundary of [`AccessKeyAuthorization`].
    #[test]
    fn parse_case_vectors() {
        let TestVectors { parse_cases, .. } = vectors();
        assert!(!parse_cases.is_empty(), "no test vectors");

        for ParseCase { name, blob, parses } in parse_cases {
            assert_eq!(
                serde_json::from_str::<AccessKeyAuthorization>(&blob).is_ok(),
                parses,
                "vector '{name}'",
            );
        }
    }

    /// Account IDs are validated on deserialization, in `signer_id` and in `path` alike.
    #[test]
    fn account_id_vectors() {
        let TestVectors {
            account_id_cases: AccountIdCases { template, cases },
            ..
        } = vectors();
        assert!(!cases.is_empty(), "no test vectors");

        for AccountIdCase {
            account_id,
            field,
            parses,
        } in cases
        {
            let mut blob: serde_json::Value =
                serde_json::from_str(&template).expect("invalid template");
            blob["msg"][&field] = match field.as_str() {
                "signer_id" => account_id.as_str().into(),
                "path" => serde_json::Value::from(vec![account_id.as_str()]),
                _ => panic!("unknown field '{field}'"),
            };

            assert_eq!(
                serde_json::from_value::<AccessKeyAuthorization>(blob).is_ok(),
                parses,
                "account ID '{account_id}' in '{field}'",
            );
        }
    }

    /// [`PublicKey::to_implicit_account_id`] on both curves.
    #[test]
    fn implicit_account_vectors() {
        let TestVectors {
            implicit_accounts, ..
        } = vectors();
        assert!(!implicit_accounts.is_empty(), "no test vectors");

        for ImplicitAccount {
            name,
            curve,
            public_key,
            account_id,
        } in implicit_accounts
        {
            let public_key: PublicKey = public_key
                .parse()
                .unwrap_or_else(|_| panic!("vector '{name}'"));
            assert_curve(&name, &curve, &public_key);
            assert_eq!(
                public_key.to_implicit_account_id().as_str(),
                account_id,
                "vector '{name}'",
            );
        }
    }
}
