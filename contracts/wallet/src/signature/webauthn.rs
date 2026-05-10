use core::marker::PhantomData;

pub use defuse_webauthn::*;
use near_sdk::{serde::de::DeserializeOwned, serde_json};

use crate::signature::SigningStandard;

/// [`WebAuthn`](https://w3c.github.io/webauthn) signing standard
pub struct Webauthn<A: Algorithm + ?Sized, D: digest::Digest>(
    PhantomData<A>,
    PhantomData<fn() -> D>,
);

impl<A: Algorithm + ?Sized, D: digest::Digest> Clone for Webauthn<A, D> {
    fn clone(&self) -> Self {
        Self(PhantomData, PhantomData)
    }
}

impl<A: Algorithm + ?Sized, D: digest::Digest> core::fmt::Debug for Webauthn<A, D> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Webauthn").finish()
    }
}

impl<A: Algorithm + ?Sized, D: digest::Digest> PartialEq for Webauthn<A, D> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl<A: Algorithm + ?Sized, D: digest::Digest> Eq for Webauthn<A, D> {}

impl<M, A, D> SigningStandard<M> for Webauthn<A, D>
where
    A: Algorithm + ?Sized,
    D: digest::Digest,
    A::Signature: DeserializeOwned,
    M: AsRef<[u8]>,
{
    type PublicKey = A::PublicKey;

    fn verify(msg: M, public_key: &Self::PublicKey, signature: &str) -> bool {
        let Ok(signature) = serde_json::from_str::<PayloadSignature<A, D>>(signature) else {
            return false;
        };

        signature.verify(msg, public_key, UserVerification::Ignore)
    }
}
