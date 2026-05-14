use defuse_outlayer_crypto::DerivableCurve;

pub trait DerivationScheme<C, P>
where
    C: DerivableCurve + ?Sized,
{
    fn tweak(path: P) -> C::Tweak;

    fn derive_public_key(master_pk: &C::PublicKey, path: P) -> C::PublicKey {
        let tweak = Self::tweak(path);

        C::derive_public_key(master_pk, &tweak)
    }
}

pub trait SubScheme<P> {
    type Output;

    fn derive(path: P) -> Self::Output;
}

pub struct Identity;

impl<T> SubScheme<T> for Identity {
    type Output = T;

    #[inline]
    fn derive(path: T) -> T {
        path
    }
}
