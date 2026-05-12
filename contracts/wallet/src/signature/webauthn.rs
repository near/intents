use core::marker::PhantomData;

pub use defuse_webauthn::*;
use near_sdk::{serde::de::DeserializeOwned, serde_json};

use crate::signature::{SigningStandard, SigningStandardPrehash};

/// [`WebAuthn`](https://w3c.github.io/webauthn) signing standard
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Webauthn<A: ?Sized>(PhantomData<A>);

impl<M, A> SigningStandard<M> for Webauthn<A>
where
    A: Algorithm + ?Sized,
    A::Signature: DeserializeOwned,
    M: AsRef<[u8]>,
{
    type PublicKey = A::PublicKey;

    fn verify(msg: M, public_key: &Self::PublicKey, signature: &str) -> bool {
        let Ok(signature) = serde_json::from_str::<PayloadSignature<A>(signature) else {
            return false;
        };

        signature.verify(msg, public_key, UserVerification::Ignore)
    }
}

impl<M, A> SigningStandardPrehash<M> for Webauthn<A>
where
    A: AlgorithmPrehash + ?Sized,
    A::Signature: DeserializeOwned,
    M: AsRef<[u8]>,
{
    type PublicKey = A::PublicKey;

    fn verify_prehash<D: digest::Digest<OutputSize = digest::consts::U32>>(
        msg: M,
        public_key: &Self::PublicKey,
        signature: &str,
    ) -> bool {
        let Ok(signature) = serde_json::from_str::<PayloadSignature<A, D>>(signature) else {
            return false;
        };

        signature.verify_prehash::<D>(msg, public_key, UserVerification::Ignore)
    }
}
