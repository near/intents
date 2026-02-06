use defuse_crypto::{Curve, Ed25519, P256, PublicKey, serde::AsCurve};
use defuse_serde_utils::base64::{Base64, Unpadded, UrlSafe};
use near_sdk::{
    env, near,
    serde::{Serialize, de::DeserializeOwned},
    serde_json,
};

// TODO: field ordering (borsh)
#[near(serializers = [borsh, json])]
#[serde(bound(
    serialize = "<A as Algorithm>::Signature: Serialize",
    deserialize = "<A as Algorithm>::Signature: DeserializeOwned",
))]
#[derive(Debug, Clone)]
pub struct PayloadSignature<A: Algorithm = Any> {
    /// Base64Url-encoded [authenticatorData](https://w3c.github.io/webauthn/#authenticator-data)
    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        schemars(with = "String")
    )]
    #[serde_as(as = "Base64<UrlSafe, Unpadded>")]
    pub authenticator_data: Vec<u8>,
    /// Serialized [clientDataJSON](https://w3c.github.io/webauthn/#dom-authenticatorresponse-clientdatajson)
    pub client_data_json: String,

    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        schemars(with = "String"),
        borsh(schema(params = "A => <A as Algorithm>::Signature"))
    )]
    // TODO: serde_as?
    pub signature: A::Signature,
}

impl<A: Algorithm> PayloadSignature<A> {
    /// <https://w3c.github.io/webauthn/#sctn-verifying-assertion>
    ///
    /// Credits to:
    /// * [ERC-4337 Smart Wallet](https://github.com/passkeys-4337/smart-wallet/blob/f3aa9fd44646fde0316fc810e21cc553a9ed73e0/contracts/src/WebAuthn.sol#L75-L172)
    /// * [CAP-0051](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0051.md)
    pub fn verify(
        &self,
        message: impl AsRef<[u8]>,
        public_key: &A::PublicKey,
        require_user_verification: bool,
    ) -> bool {
        // verify authData flags
        if self.authenticator_data.len() < 37
            || !Self::verify_flags(self.authenticator_data[32], require_user_verification)
        {
            return false;
        }

        // 10. Verify that the value of C.type is the string webauthn.get.
        let Ok(c) = serde_json::from_str::<CollectedClientData>(&self.client_data_json) else {
            return false;
        };
        if c.typ != ClientDataType::Get {
            return false;
        }

        // 11. Verify that the value of C.challenge equals the base64url
        // encoding of pkOptions.challenge
        //
        // In our case, challenge is a hash of the payload
        if c.challenge != message.as_ref() {
            return false;
        }

        // 20. Let hash be the result of computing a hash over the cData using
        // SHA-256
        let hash = env::sha256_array(self.client_data_json.as_bytes());

        // 21. Using credentialRecord.publicKey, verify that sig is a valid
        // signature over the binary concatenation of authData and hash.
        A::verify(
            &[self.authenticator_data.as_slice(), hash.as_slice()].concat(),
            public_key,
            &self.signature,
        )
    }

    #[allow(clippy::identity_op)]
    const AUTH_DATA_FLAGS_UP: u8 = 1 << 0;
    const AUTH_DATA_FLAGS_UV: u8 = 1 << 2;
    const AUTH_DATA_FLAGS_BE: u8 = 1 << 3;
    const AUTH_DATA_FLAGS_BS: u8 = 1 << 4;

    /// <https://w3c.github.io/webauthn/#sctn-verifying-assertion>
    const fn verify_flags(flags: u8, require_user_verification: bool) -> bool {
        // 16. Verify that the UP bit of the flags in authData is set.
        if flags & Self::AUTH_DATA_FLAGS_UP != Self::AUTH_DATA_FLAGS_UP {
            return false;
        }

        // 17. If user verification was determined to be required, verify that
        // the UV bit of the flags in authData is set. Otherwise, ignore the
        // value of the UV flag.
        if require_user_verification
            && (flags & Self::AUTH_DATA_FLAGS_UV != Self::AUTH_DATA_FLAGS_UV)
        {
            return false;
        }

        // 18. If the BE bit of the flags in authData is not set, verify that
        // the BS bit is not set.
        if (flags & Self::AUTH_DATA_FLAGS_BE != Self::AUTH_DATA_FLAGS_BE)
            && (flags & Self::AUTH_DATA_FLAGS_BS == Self::AUTH_DATA_FLAGS_BS)
        {
            return false;
        }

        true
    }
}

/// For more details, refer to [WebAuthn specification](https://w3c.github.io/webauthn/#dictdef-collectedclientdata).
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct CollectedClientData {
    #[serde(rename = "type")]
    pub typ: ClientDataType,

    #[serde_as(as = "Base64<UrlSafe, Unpadded>")]
    pub challenge: Vec<u8>,

    pub origin: String,
}

#[near(serializers = [json])]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientDataType {
    /// Serializes to the string `"webauthn.create"`
    #[serde(rename = "webauthn.create")]
    Create,

    /// Serializes to the string `"webauthn.get"`
    #[serde(rename = "webauthn.get")]
    Get,
}

#[near(serializers = [json])]
#[serde(untagged)]
#[derive(Debug, Clone)]
pub enum Signature {
    /// [COSE EdDSA (-8) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
    /// ed25519 curve
    Ed25519(#[serde_as(as = "AsCurve<Ed25519>")] <Ed25519 as Curve>::Signature),
    /// [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms): NIST P-256 curve (a.k.a secp256r1) over SHA-256
    P256(#[serde_as(as = "AsCurve<P256>")] <P256 as Curve>::Signature),
}

// impl Signature {
//     #[inline]
//     pub fn verify(&self, message: &[u8], public_key: &PublicKey) -> bool {
//         match (self, public_key) {
//             // [COSE EdDSA (-8) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
//             // ed25519 curve
//             (Self::Ed25519(signature), PublicKey::Ed25519(public_key)) => {
//                 Ed25519::verify(signature, message, public_key).is_some()
//             }
//             // [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
//             // P256 (a.k.a secp256r1) over SHA-256
//             (Self::P256(signature), PublicKey::P256(public_key)) => {
//                 // Use host impl of SHA-256 here to reduce gas consumption
//                 let prehashed = env::sha256_array(message);
//                 P256::verify(signature, &prehashed, public_key).is_some()
//             }
//             _ => false,
//         }
//     }
// }

/// https://www.iana.org/assignments/cose/cose.xhtml#algorithms
pub trait Algorithm {
    type PublicKey;
    type Signature;

    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &Self::Signature) -> bool;
}

/// [COSE EdDSA (-8) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// ed25519 curve
#[derive(Debug, Clone)]
pub struct EdDSA;

impl Algorithm for EdDSA {
    type PublicKey = <Ed25519 as Curve>::PublicKey;
    type Signature = <Ed25519 as Curve>::Signature;

    #[inline]
    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &Self::Signature) -> bool {
        Ed25519::verify(signature, msg, public_key).is_some()
    }
}

/// [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// P256 (a.k.a secp256r1) over SHA-256
pub struct ES256;

impl Algorithm for ES256 {
    type PublicKey = <P256 as Curve>::PublicKey;
    type Signature = <P256 as Curve>::Signature;

    #[inline]
    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &Self::Signature) -> bool {
        // Use host impl of SHA-256 here to reduce gas consumption
        let prehashed = env::sha256_array(msg);
        P256::verify(signature, &prehashed, public_key).is_some()
    }
}

// TODO: rename
#[derive(Debug, Clone)]
pub struct Any;

impl Algorithm for Any {
    type PublicKey = PublicKey;

    type Signature = Signature;

    #[inline]
    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &Self::Signature) -> bool {
        match (public_key, signature) {
            (PublicKey::Ed25519(public_key), Signature::Ed25519(signature)) => {
                EdDSA::verify(msg, public_key, signature)
            }

            (PublicKey::P256(public_key), Signature::P256(signature)) => {
                ES256::verify(msg, public_key, signature)
            }
            _ => false,
        }
    }
}
