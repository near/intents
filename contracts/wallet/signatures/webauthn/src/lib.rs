#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "p256")]
pub mod p256;

pub use defuse_webauthn as webauthn;

use core::marker::PhantomData;

use defuse_crypto::Curve;
use defuse_wallet::{RequestMessage, SignatureSchema};
use defuse_webauthn::{Algorithm, UserVerification, Webauthn, WebauthnAssertion};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Webauthn wallet [signature schema](SignatureSchema).
///
/// See [`Webauthn`] for more.
pub struct WalletWebauthn<A: Algorithm, UV: UserVerification>(PhantomData<Webauthn<A, UV>>);

impl<A, UV> SignatureSchema for WalletWebauthn<A, UV>
where
    A: WalletWebauthnAlgorithm,
    UV: UserVerification,
    <A::Curve as Curve>::Signature: TryFrom<A::Signature>,
    for<'a> <A::Curve as Curve>::PublicKey: TryFrom<&'a A::PublicKey>,
{
    type PublicKey = A::PublicKey;

    fn verify(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool {
        // try to convert public key
        let Ok(public_key) = <A::Curve as Curve>::PublicKey::try_from(public_key) else {
            return false;
        };

        // try to deserialize proof
        let Ok(proof) = serde_json::from_str::<WalletWebauthnProof<A::Signature>>(proof) else {
            return false;
        };

        // try to convert signature
        let Ok(signature) = <A::Curve as Curve>::Signature::try_from(proof.signature) else {
            return false;
        };

        // Verify `msg.hash()` according to webauthn spec.
        //
        // We `msg.hash()` as the challenge, since:
        // * Authenticators are general-purpose signers and they usually
        //   implement blind singing.
        // * This reduces length of the `proof` submitted on-chain.
        Webauthn::<A, UV>::verify(&public_key, msg.hash(), &proof.assertion, &signature)
    }
}

/// Adaptor for [`Algorithm`] used by [`WalletWebauthn`] to implement
/// [`SignatureSchema`]
pub trait WalletWebauthnAlgorithm: Algorithm {
    /// Used as [`SignatureSchema::PublicKey`].
    type PublicKey;

    /// Used as [`WalletWebauthnProof::signature`]
    type Signature: DeserializeOwned;
}

/// JSON proof used by [`WalletWebauthn`]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletWebauthnProof<S> {
    /// Signed assertion
    #[serde(flatten)]
    pub assertion: WebauthnAssertion,

    /// [`Algorithm`]-specific signature
    pub signature: S,
}
