use defuse_crypto::{Curve, Ed25519, P256, PublicKey, serde::AsCurve};
use defuse_serde_utils::base64::{Base64, Unpadded, UrlSafe};
use near_sdk::{env, near, serde_json};
use serde_with::serde_as;

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct PayloadSignature {
    /// Base64Url-encoded [authenticatorData](https://w3c.github.io/webauthn/#authenticator-data)
    #[serde_as(as = "Base64<UrlSafe, Unpadded>")]
    pub authenticator_data: Vec<u8>,
    /// Serialized [clientDataJSON](https://w3c.github.io/webauthn/#dom-authenticatorresponse-clientdatajson)
    pub client_data_json: String,

    #[serde(flatten)]
    pub signature: Signature,
}

impl PayloadSignature {
    /// <https://w3c.github.io/webauthn/#sctn-verifying-assertion>
    ///
    /// Credits to:
    /// * [ERC-4337 Smart Wallet](https://github.com/passkeys-4337/smart-wallet/blob/f3aa9fd44646fde0316fc810e21cc553a9ed73e0/contracts/src/WebAuthn.sol#L75-L172)
    /// * [CAP-0051](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0051.md)
    pub fn verify(
        &self,
        message: impl AsRef<[u8]>,
        require_user_verification: bool,
    ) -> Option<PublicKey> {
        // verify authData flags
        if self.authenticator_data.len() < 37
            || !Self::verify_flags(self.authenticator_data[32], require_user_verification)
        {
            return None;
        }

        // 10. Verify that the value of C.type is the string webauthn.get.
        let c: CollectedClientData = serde_json::from_str(&self.client_data_json).ok()?;
        if c.typ != ClientDataType::Get {
            return None;
        }

        // 11. Verify that the value of C.challenge equals the base64url
        // encoding of pkOptions.challenge
        //
        // In our case, challenge is a hash of the payload
        if c.challenge != message.as_ref() {
            return None;
        }

        // 20. Let hash be the result of computing a hash over the cData using
        // SHA-256
        let hash = env::sha256_array(self.client_data_json.as_bytes());

        // 21. Using credentialRecord.publicKey, verify that sig is a valid
        // signature over the binary concatenation of authData and hash.
        self.signature
            .verify(&[self.authenticator_data.as_slice(), hash.as_slice()].concat())
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
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
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

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[serde(untagged)]
#[derive(Debug, Clone)]
pub enum Signature {
    /// [COSE EdDSA (-8) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
    /// ed25519 curve
    Ed25519 {
        #[serde_as(as = "AsCurve<Ed25519>")]
        public_key: <Ed25519 as Curve>::PublicKey,
        #[serde_as(as = "AsCurve<Ed25519>")]
        signature: <Ed25519 as Curve>::Signature,
    },
    /// [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms): NIST P-256 curve (a.k.a secp256r1) over SHA-256
    P256 {
        #[serde_as(as = "AsCurve<P256>")]
        public_key: <P256 as Curve>::PublicKey,
        #[serde_as(as = "AsCurve<P256>")]
        signature: <P256 as Curve>::Signature,
    },
}

impl Signature {
    #[inline]
    pub fn verify(&self, message: &[u8]) -> Option<PublicKey> {
        match self {
            // [COSE EdDSA (-8) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
            // ed25519 curve
            Self::Ed25519 {
                public_key,
                signature,
            } => Ed25519::verify(signature, message, public_key).map(PublicKey::Ed25519),
            // [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
            // P256 (a.k.a secp256r1) over SHA-256
            Self::P256 {
                public_key,
                signature,
            } => {
                // Use host impl of SHA-256 here to reduce gas consumption
                let prehashed = env::sha256_array(message);
                P256::verify(signature, &prehashed, public_key).map(PublicKey::P256)
            }
        }
    }
}
