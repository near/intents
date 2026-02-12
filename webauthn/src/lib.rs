#[cfg(feature = "ed25519")]
mod ed25519;
#[cfg(feature = "ed25519")]
pub use self::ed25519::*;

#[cfg(feature = "p256")]
mod p256;
#[cfg(feature = "p256")]
pub use self::p256::*;

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
pub struct PayloadSignature<A: Algorithm + ?Sized> {
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
        borsh(schema(params = "A => <A as Algorithm>::Signature")),
        // schemars@0.8 does not respect it's `schemars(bound = "...")`
        // attribute: https://github.com/GREsau/schemars/blob/104b0fd65055d4b46f8dcbe38cdd2ef2c4098fe2/schemars_derive/src/lib.rs#L193-L206
        schemars(with = "String"),
    )]
    pub signature: A::Signature,
}

impl<A: Algorithm + ?Sized> PayloadSignature<A> {
    /// <https://w3c.github.io/webauthn/#sctn-verifying-assertion>
    ///
    /// Credits to:
    /// * [ERC-4337 Smart Wallet](https://github.com/passkeys-4337/smart-wallet/blob/f3aa9fd44646fde0316fc810e21cc553a9ed73e0/contracts/src/WebAuthn.sol#L75-L172)
    /// * [CAP-0051](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0051.md)
    pub fn verify(
        &self,
        message: impl AsRef<[u8]>,
        public_key: &A::PublicKey,
        user_verification: UserVerification,
    ) -> bool {
        // verify authData flags
        if self.authenticator_data.len() < 37
            || !Self::verify_flags(self.authenticator_data[32], user_verification)
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
    const fn verify_flags(flags: u8, user_verification: UserVerification) -> bool {
        // 16. Verify that the UP bit of the flags in authData is set.
        if flags & Self::AUTH_DATA_FLAGS_UP != Self::AUTH_DATA_FLAGS_UP {
            return false;
        }

        // 17. If user verification was determined to be required, verify that
        // the UV bit of the flags in authData is set. Otherwise, ignore the
        // value of the UV flag.
        if user_verification.is_required()
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

#[derive(Debug, Clone, Copy)]
pub enum UserVerification {
    Ignore,
    Require,
}

impl UserVerification {
    #[inline]
    pub const fn is_required(&self) -> bool {
        matches!(self, Self::Require)
    }
}

/// See <https://www.iana.org/assignments/cose/cose.xhtml#algorithms>
pub trait Algorithm {
    type PublicKey;
    type Signature;

    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &Self::Signature) -> bool;
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
