pub mod ed25519;
pub mod secp256k1;
#[cfg(feature = "signing")]
pub mod signer;

pub trait DerivableCurve {
    type Tweak;
    type Signature;

    fn tweak(hash: [u8; 32]) -> Self::Tweak;
    // TODO: verify()?
}

pub trait DerivablePublicKey<C>: Sized
where
    C: DerivableCurve,
{
    #[must_use]
    fn derive(&self, tweak: C::Tweak) -> Self;
}

#[cfg(feature = "signing")]
#[impl_tools::autoimpl(for<T: trait + ?Sized>
    &T,
    &mut T,
    Box<T>,
    std::rc::Rc<T>,
    std::sync::Arc<T>
)]
pub trait DeriveSigner<C>
where
    C: DerivableCurve,
{
    type PublicKey: DerivablePublicKey<C>;

    fn public_key(&self) -> Self::PublicKey;

    fn sign(&self, tweak: C::Tweak, msg: &[u8]) -> C::Signature;
}

#[cfg(all(test, feature = "signing"))]
mod tests {
    use sha3::{Digest, Sha3_256};

    use super::*;

    pub fn test_roundtrip<K, C>(
        root_sk: K,
        verify: impl FnOnce(K::PublicKey, &[u8], <C as DerivableCurve>::Signature),
    ) where
        K: DeriveSigner<C>,
        C: DerivableCurve,
        <C as DerivableCurve>::Tweak: Copy,
    {
        let tweak = <C as DerivableCurve>::tweak([42u8; 32]);
        let derived_pk = root_sk.public_key().derive(tweak);

        // TODO: type-safe msg or prehash?
        let msg: [u8; 32] = Sha3_256::digest(b"message").into();

        let signature = root_sk.sign(tweak, &msg);

        verify(derived_pk, &msg, signature);
    }
}
