use std::marker::PhantomData;

pub use defuse_webauthn::*;
use near_sdk::{
    borsh::{self, BorshDeserialize},
    env,
};

use crate::SigningStandard;

pub struct Webauthn<A: Algorithm>(PhantomData<A>);

impl<A: Algorithm> SigningStandard for Webauthn<A>
where
    A::Signature: BorshDeserialize,
{
    type PublicKey = A::PublicKey;

    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &[u8]) -> bool {
        let Ok(signature) = borsh::from_slice::<PayloadSignature<A>>(signature) else {
            return false;
        };

        let hash = env::sha256_array(msg);
        signature.verify(hash, public_key, false)
    }
}
