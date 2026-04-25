pub mod ed25519;
pub mod secp256k1;
pub mod signer;

use std::{rc::Rc, sync::Arc};

use borsh::BorshSerialize;
use digest::Digest;
use digest_io::IoWrapper;
use impl_tools::autoimpl;
use sha3::Sha3_256;

pub trait DerivableCurve {
    type Tweak;
    type Signature;

    fn make_tweak(tweak: [u8; 32]) -> Self::Tweak;

    fn derive_tweak(path: impl BorshSerialize) -> Self::Tweak {
        let mut hasher = IoWrapper(Sha3_256::new());
        borsh::to_writer(&mut hasher, &path).expect("borsh");
        Self::make_tweak(hasher.0.finalize().into())
    }
}

pub trait DerivablePublicKey<C>: Sized
where
    C: DerivableCurve,
{
    #[must_use]
    fn derive_from_tweak(&self, tweak: C::Tweak) -> Self;

    #[must_use]
    fn derive_from_borsh(&self, path: impl BorshSerialize) -> Self {
        self.derive_from_tweak(C::derive_tweak(path))
    }
}

#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
pub trait DerivableSigningKey<C>
where
    C: DerivableCurve,
{
    type PublicKey: DerivablePublicKey<C>;

    fn public_key(&self) -> Self::PublicKey;

    fn sign_derive_from_tweak(&self, tweak: C::Tweak, msg: &[u8]) -> C::Signature;
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::*;

    pub fn test_roundtrip<K, C>(
        root_sk: K,
        verify: impl FnOnce(K::PublicKey, &[u8], <C as DerivableCurve>::Signature),
    ) where
        K: DerivableSigningKey<C>,
        C: DerivableCurve,
        <C as DerivableCurve>::Tweak: Copy,
    {
        let tweak = <C as DerivableCurve>::derive_tweak(());
        let derived_pk = root_sk.public_key().derive_from_tweak(tweak);

        // TODO: type-safe msg or prehash?
        let msg: [u8; 32] = Sha256::digest(b"message").into();

        let signature = root_sk.sign_derive_from_tweak(tweak, &msg);

        verify(derived_pk, &msg, signature);
    }
}
