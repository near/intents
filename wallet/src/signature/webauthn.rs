use core::marker::PhantomData;

pub use defuse_webauthn::*;
use near_sdk::{env, serde::de::DeserializeOwned, serde_json};

use crate::SigningStandard;

#[derive(Debug)]
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

        let hash = env::sha256_array(msg); // TODO: or keccak256?
        signature.verify(hash, public_key, UserVerification::Ignore)
    }
}
