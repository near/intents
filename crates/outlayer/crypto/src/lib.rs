#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "secp256k1")]
pub mod secp256k1;
#[cfg(feature = "signing")]
pub mod signer;

/// A curve with **non-hardened** public key derivation capabilities,
/// i.e. when child public key can be derived from the root one
/// without knowing any secret.
///
/// The derivation is **non-hierarchical** (or "plain"): derived
/// keys **do not** form a tree-like structure. Instead, child keys
/// are all derived from a single root key and can be considered as
/// "peers" to each other.
pub trait DerivableCurve {
    /// An intermediate result of [derivation](Self::tweak) that is
    /// reused for both public and signing key derivation.
    type Tweak;

    /// Public key of the curve
    type PublicKey;

    /// Message for signing.
    ///
    /// This type can vary between different [`DerivableCurve`] implementatons:
    /// some curves can sign arbitrary byte slices, while others might expect
    /// prehashed messages of a specific size.
    type Message: ?Sized;

    /// Signature of the curve
    type Signature;

    /// Derive curve-specific [tweak](Self::Tweak) from a **uniform**
    /// digest.
    fn tweak(hash: [u8; 32]) -> Self::Tweak;

    /// Derive public key from root for given [tweak](Self::Tweak).
    fn derive_public_key(root: &Self::PublicKey, tweak: &Self::Tweak) -> Self::PublicKey;

    /// Verify the signature over the message for given public key
    fn verify(
        public_key: &Self::PublicKey,
        msg: &Self::Message,
        signature: &Self::Signature,
    ) -> bool;
}

#[cfg(feature = "signing")]
#[impl_tools::autoimpl(for<T: trait + ?Sized>
    &T,
    &mut T,
    Box<T>,
    std::rc::Rc<T>,
    std::sync::Arc<T>
)]
#[impl_tools::autoimpl(
    for<T: trait + ToOwned + ?Sized>
    std::borrow::Cow<'_, T>
)]
/// Signer for [`DerivableCurve`]
pub trait DeriveSigner<C>
where
    C: DerivableCurve,
{
    /// Get root public key of the signer
    fn public_key(&self) -> C::PublicKey;

    /// Sign given message with a secret key **internally** derived for given
    /// [tweak](DerivableCurve::Tweak).
    ///
    /// NOTE: the returned signatures are non-deterministic, i.e.
    /// implementations MAY return different signatures for the same
    /// `tweak` and `msg`.
    fn sign(&self, tweak: &C::Tweak, msg: &C::Message) -> C::Signature;

    /// Helper method to derive public key from [root](Self::public_key)
    /// for given [tweak](DerivableCurve::Tweak)
    fn derive_public_key(&self, tweak: &C::Tweak) -> C::PublicKey {
        C::derive_public_key(&self.public_key(), tweak)
    }
}

#[cfg(all(test, feature = "signing"))]
mod tests {
    use std::fmt::Debug;

    use super::*;

    #[track_caller]
    pub fn assert_roundtrip<C, S>(
        root_sk: S,
        tweak: [u8; 32],
        msg: &C::Message,
    ) -> (C::PublicKey, C::Signature)
    where
        C: DerivableCurve,
        S: DeriveSigner<C>,
    {
        let tweak = C::tweak(tweak);
        let derived_pk = C::derive_public_key(&root_sk.public_key(), &tweak);
        let signature = root_sk.sign(&tweak, msg);

        assert!(C::verify(&derived_pk, msg, &signature), "invalid signature");

        (derived_pk, signature)
    }

    #[track_caller]
    pub fn assert_roundtrip_expected<C, S>(
        root_sk: S,
        tweak: [u8; 32],
        msg: &C::Message,
        expected_derived_pk: &C::PublicKey,
    ) -> C::Signature
    where
        C: DerivableCurve,
        C::PublicKey: PartialEq + Debug,
        S: DeriveSigner<C>,
    {
        let (derived_pk, signature) = assert_roundtrip(root_sk, tweak, msg);
        assert_eq!(
            &derived_pk, expected_derived_pk,
            "derived public key has changed"
        );
        signature
    }
}
