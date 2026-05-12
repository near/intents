use core::marker::PhantomData;

pub use defuse_webauthn::*;
use near_sdk::{serde::de::DeserializeOwned, serde_json};

use crate::signature::SigningStandard;
use defuse_near_utils::digest::Sha256 as NearSha256;

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
        let Ok(signature) = serde_json::from_str::<PayloadSignature<A>>(signature) else {
            return false;
        };

        signature.verify::<NearSha256>(msg, public_key, UserVerification::Ignore)
    }
}
