use core::marker::PhantomData;

use defuse_digest::Digest;

use crate::signature::SigningStandard;

/// [`SigningStandard`] middleware that forwards SHA-256 hash of the message
/// to the underlying signing standard `S`
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
        S::verify(
            defuse_digest::sha2::Sha256::digest(msg).into(),
            public_key,
            signature,
        )
    }
}

/// [`SigningStandard`] middleware that forwards keccak256 hash of the message
/// to the underlying signing standard `S`
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
        S::verify(
            defuse_digest::sha3::Keccak256::digest(msg).into(),
            public_key,
            signature,
        )
    }
}

/// [`SigningStandard`] middleware that forwards SHA3-256 hash of the message
/// to the underlying signing standard `S`
pub struct Sha3_256<S>(PhantomData<S>)
where
    S: SigningStandard<[u8; 32]> + ?Sized;

impl<M, S> SigningStandard<M> for Sha3_256<S>
where
    S: SigningStandard<[u8; 32]> + ?Sized,
    M: AsRef<[u8]>,
{
    type PublicKey = S::PublicKey;

    fn verify(msg: M, public_key: &Self::PublicKey, signature: &str) -> bool {
        S::verify(
            defuse_digest::sha3::Sha3_256::digest(msg).into(),
            public_key,
            signature,
        )
    }
}
