use std::marker::PhantomData;

use near_sdk::borsh::{self, BorshSerialize};

use crate::signature::SigningStandard;

/// [`SigningStandard`] middleware that forwards message serialized as borsh
/// to the underlying signing standard `S`
pub struct Borsh<S>(PhantomData<S>)
where
    S: SigningStandard<Vec<u8>> + ?Sized;

impl<M, S> SigningStandard<M> for Borsh<S>
where
    S: SigningStandard<Vec<u8>> + ?Sized,
    M: BorshSerialize,
{
    type PublicKey = <S as SigningStandard<Vec<u8>>>::PublicKey;

    fn verify(msg: M, public_key: &Self::PublicKey, signature: &str) -> bool {
        let Ok(serialized) = borsh::to_vec(&msg) else {
            return false;
        };

        S::verify(serialized, public_key, signature)
    }
}
