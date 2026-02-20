use core::marker::PhantomData;

use crate::signature::SigningStandard;

/// Domain prefix for the wallet-contract
pub const WALLET_DOMAIN: &[u8] = b"NEAR_WALLET_CONTRACT/V1";

pub struct DomainPrefix<S>(PhantomData<S>)
where
    S: SigningStandard<Vec<u8>> + ?Sized;

impl<M, S> SigningStandard<M> for DomainPrefix<S>
where
    S: SigningStandard<Vec<u8>> + ?Sized,
    M: AsRef<[u8]>,
{
    type PublicKey = S::PublicKey;

    fn verify(msg: M, public_key: &Self::PublicKey, signature: &str) -> bool {
        S::verify(
            [WALLET_DOMAIN, msg.as_ref()].concat(),
            public_key,
            signature,
        )
    }
}
