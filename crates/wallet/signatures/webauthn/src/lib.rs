use core::{fmt::Display, marker::PhantomData};

use borsh::{BorshDeserialize, BorshSerialize};
use defuse_kdf_crypto::Curve;
use defuse_wallet_core::{RequestMessage, signatures::WalletSignatureSchema};
use defuse_webauthn::{Algorithm, UserVerification, Webauthn, WebauthnPayload};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub struct WalletWebauthn<A: Algorithm, UV: UserVerification>(PhantomData<Webauthn<A, UV>>);

impl<A, UV> WalletSignatureSchema for WalletWebauthn<A, UV>
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

        Webauthn::<A, UV>::verify(&public_key, msg.hash(), &proof.payload, &signature)
    }
}

pub trait WalletWebauthnAlgorithm: Algorithm {
    type PublicKey: BorshSerialize + BorshDeserialize + Display;
    type Signature;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletWebauthnProof<S> {
    #[serde(flatten)]
    pub payload: WebauthnPayload,

    pub signature: S,
}
