use core::marker::PhantomData;

pub use defuse_webauthn::*;
use near_sdk::{serde::de::DeserializeOwned, serde_json};

use crate::SigningStandard;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Webauthn<A: Algorithm>(PhantomData<A>);

impl<A: Algorithm> SigningStandard for Webauthn<A>
where
    A::Signature: DeserializeOwned,
{
    type PublicKey = A::PublicKey;

    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &str) -> bool {
        let Ok(signature) = serde_json::from_str::<PayloadSignature<A>>(signature) else {
            return false;
        };

        signature.verify(msg, public_key, UserVerification::Ignore)
    }
}
