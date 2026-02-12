use core::marker::PhantomData;

use near_sdk::env;

use crate::signature::SigningStandard;

pub struct Sha256<S>(PhantomData<S>)
where
    S: SigningStandard<[u8; 32]> + ?Sized;

impl<M, S> SigningStandard<M> for Sha256<S>
where
    S: SigningStandard<[u8; 32]> + ?Sized,
    M: AsRef<[u8]>,
{
    type PublicKey = S::PublicKey;

    fn verify(msg: M, public_key: &Self::PublicKey, signature: &str) -> bool {
        S::verify(env::sha256_array(msg), public_key, signature)
    }
}

pub struct Keccak256<S>(PhantomData<S>)
where
    S: SigningStandard<[u8; 32]> + ?Sized;

impl<M, S> SigningStandard<M> for Keccak256<S>
where
    S: SigningStandard<[u8; 32]> + ?Sized,
    M: AsRef<[u8]>,
{
    type PublicKey = S::PublicKey;

    fn verify(msg: M, public_key: &Self::PublicKey, signature: &str) -> bool {
        S::verify(env::keccak256_array(msg), public_key, signature)
    }
}
