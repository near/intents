#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "p256")]
pub mod p256;

use core::marker::PhantomData;

use defuse_crypto::Curve;
use defuse_wallet::{RequestMessage, SignatureSchema};
pub use defuse_webauthn as webauthn;
use defuse_webauthn::{Algorithm, UserVerification, Webauthn, WebauthnAssertion};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

// TODO: docs
pub struct WalletWebauthn<A: Algorithm, UV: UserVerification>(PhantomData<Webauthn<A, UV>>);

impl<A, UV> SignatureSchema for WalletWebauthn<A, UV>
where
    A: WalletWebauthnAlgorithm<Signature: DeserializeOwned>,
    UV: UserVerification,
    <A::Curve as Curve>::Signature: TryFrom<A::Signature>,
    for<'a> <A::Curve as Curve>::PublicKey: TryFrom<&'a A::PublicKey>,
{
    type PublicKey = A::PublicKey;

    fn verify(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool {
        let Ok(public_key) = <A::Curve as Curve>::PublicKey::try_from(public_key) else {
            return false;
        };

        let Ok(proof) = serde_json::from_str::<WalletWebauthnProof<A::Signature>>(proof) else {
            return false;
        };

        let Ok(signature) = <A::Curve as Curve>::Signature::try_from(proof.signature) else {
            return false;
        };

        // We hash the payload for webauthn, since:
        // 1. Authenticators are general-purpose signers and they usually
        //    implement blind singing.
        // 2. This reduces length of the `proof` submitted on-chain.
        Webauthn::<A, UV>::verify(&public_key, msg.hash(), &proof.payload, &signature)
    }
}

pub trait WalletWebauthnAlgorithm: Algorithm {
    type PublicKey;
    type Signature;
}

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletWebauthnProof<S> {
    #[serde(flatten)]
    pub payload: WebauthnAssertion,

    pub signature: S,
}
