use core::marker::PhantomData;

use defuse_crypto::Curve;
use defuse_digest::{Digest, sha2::Sha256};
use serde::{Deserialize, Serialize};
use serde_with::{
    base64::{Base64, UrlSafe},
    formats::Unpadded,
    serde_as,
};

#[cfg(feature = "ed25519")]
pub mod ed25519;

#[cfg(feature = "p256")]
pub mod p256;

/// [Webauthn](https://w3c.github.io/webauthn/) signing standard generic over
/// underlying [`Algorithm`]
pub struct Webauthn<A: Algorithm, UV: UserVerification> {
    _algorithm: PhantomData<A>,
    _user_verification: PhantomData<UV>,
}

impl<A, UV> Webauthn<A, UV>
where
    A: Algorithm,
    UV: UserVerification,
{
    /// Check that given assertion corresponds to `msg` and verify the
    /// signature over it for given public key.
    ///
    /// See <https://w3c.github.io/webauthn/#sctn-verifying-assertion>.
    ///
    /// Credits to:
    /// * [ERC-4337 Smart Wallet](https://github.com/passkeys-4337/smart-wallet/blob/f3aa9fd44646fde0316fc810e21cc553a9ed73e0/contracts/src/WebAuthn.sol#L75-L172)
    /// * [CAP-0051](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0051.md)
    pub fn verify(
        public_key: &<A::Curve as Curve>::PublicKey,
        challenge: impl AsRef<[u8]>,
        assertion: &WebauthnAssertion,
        signature: &<A::Curve as Curve>::Signature,
    ) -> bool {
        if !Self::check(challenge, assertion) {
            return false;
        }

        // 20. Let hash be the result of computing a hash over the cData using
        // SHA-256
        let hash = Sha256::digest(assertion.client_data_json.as_bytes());

        // 21. Using credentialRecord.publicKey, verify that sig is a valid
        // signature over the binary concatenation of authData and hash.
        A::verify(
            public_key,
            [assertion.authenticator_data.as_slice(), hash.as_ref()].concat(),
            signature,
        )
    }

    /// Check the assertion and whether is corresponds to given `challenge`.
    fn check(challenge: impl AsRef<[u8]>, p: &WebauthnAssertion) -> bool {
        // check authData flags before `clientDataJSON` to save gas
        if p.authenticator_data.len() < 37 || !Self::check_flags(p.authenticator_data[32]) {
            return false;
        }

        // 10. Verify that the value of C.type is the string webauthn.get.
        let Ok(c) = serde_json::from_str::<CollectedClientData>(&p.client_data_json) else {
            return false;
        };
        if c.typ != ClientDataType::Get {
            return false;
        }

        // 11. Verify that the value of C.challenge equals the base64url
        // encoding of pkOptions.challenge
        if c.challenge != challenge.as_ref() {
            return false;
        }

        true
    }

    #[allow(clippy::identity_op)]
    const AUTH_DATA_FLAGS_UP: u8 = 1 << 0;
    const AUTH_DATA_FLAGS_UV: u8 = 1 << 2;
    const AUTH_DATA_FLAGS_BE: u8 = 1 << 3;
    const AUTH_DATA_FLAGS_BS: u8 = 1 << 4;

    /// Check flags in authData.
    ///
    /// See <https://w3c.github.io/webauthn/#sctn-verifying-assertion>.
    const fn check_flags(flags: u8) -> bool {
        // 16. Verify that the UP bit of the flags in authData is set.
        if flags & Self::AUTH_DATA_FLAGS_UP != Self::AUTH_DATA_FLAGS_UP {
            return false;
        }

        // 17. If user verification was determined to be required, verify that
        // the UV bit of the flags in authData is set. Otherwise, ignore the
        // value of the UV flag.
        if UV::REQUIRED && (flags & Self::AUTH_DATA_FLAGS_UV != Self::AUTH_DATA_FLAGS_UV) {
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

/// Actual payload signed according to [`Webauthn`] standard
#[serde_as]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebauthnAssertion {
    #[serde_as(as = "Base64<UrlSafe, Unpadded>")]
    #[serde(alias = "authenticatorData")]
    /// Base64Url-encoded [authenticatorData](https://w3c.github.io/webauthn/#authenticator-data)
    pub authenticator_data: Vec<u8>,

    /// Serialized [clientDataJSON](https://w3c.github.io/webauthn/#dom-authenticatorresponse-clientdatajson)
    #[serde(alias = "clientDataJSON")]
    pub client_data_json: String,
}

/// [`CollectedClientData`](https://w3c.github.io/webauthn/#dictdef-collectedclientdata)
#[serde_as]
#[cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectedClientData {
    #[serde(rename = "type")]
    pub typ: ClientDataType,

    #[serde_as(as = "Base64<UrlSafe, Unpadded>")]
    pub challenge: Vec<u8>,

    pub origin: String,
}

/// [Type](https://w3c.github.io/webauthn/#dom-collectedclientdata-type) of
/// [`CollectedClientData`]
#[cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientDataType {
    /// Serializes to the string `"webauthn.create"`
    #[serde(rename = "webauthn.create")]
    Create,

    /// Serializes to the string `"webauthn.get"`
    #[serde(rename = "webauthn.get")]
    Get,
}

/// [User verification](https://w3c.github.io/webauthn/#user-verification) mode.
///
/// # Compatibility
///
/// `UV` (User Verified) flag is only set by FIDO2-capable devices with
/// PIN / biometric setup.
///
/// FIDO U2F (CTAP 1) authenticators (such as old Ledger and Yubikey
/// devices) only set `UP` (User Present) flag and doesn't support `UV`
/// (User Verified).
pub trait UserVerification {
    const REQUIRED: bool;
}

/// Ignore [user verification](https://w3c.github.io/webauthn/#user-verification)
pub struct IgnoreUserVerification;
impl UserVerification for IgnoreUserVerification {
    const REQUIRED: bool = false;
}

/// Require [user verification](https://w3c.github.io/webauthn/#user-verification)
pub struct RequireUserVerification;
impl UserVerification for RequireUserVerification {
    const REQUIRED: bool = true;
}

/// Signature algorithm.
///
/// See <https://www.iana.org/assignments/cose/cose.xhtml#algorithms>
pub trait Algorithm {
    type Curve: Curve;

    /// Optionally, prehash the message (or perform any other manipulations)
    /// before passing it to [`Self::Curve::verify()`](Curve::verify) in the
    /// last signature verification step:
    ///
    /// > 21. Using credentialRecord.publicKey, verify that sig is a valid
    /// > signature over the binary concatenation of authData and hash.
    fn preprocess(msg: impl AsRef<[u8]>) -> impl AsRef<[u8]>;

    /// Last algorithm-specific signature verification step:
    ///
    /// > 21. Using credentialRecord.publicKey, verify that sig is a valid
    /// > signature over the binary concatenation of authData and hash.
    #[inline]
    fn verify(
        public_key: &<Self::Curve as Curve>::PublicKey,
        msg: impl AsRef<[u8]>,
        signature: &<Self::Curve as Curve>::Signature,
    ) -> bool {
        // optionally prehash the message (or perform any other manipulations)
        let msg = Self::preprocess(msg);

        // verify using `Self::Curve`
        Self::Curve::verify(public_key, msg.as_ref(), signature)
    }
}
