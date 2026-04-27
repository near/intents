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
    /// A path to derive both [public](DerivablePublicKey::derive) and
    /// [signing](DeriveSigner::derive_sign) keys for.
    /// Typically, it should be an output of a cryptographic hash function.
    type Path: ?Sized;

    /// Public key of the curve
    type PublicKey: DerivablePublicKey<Self>;

    /// Message for signing.
    ///
    /// This type can vary between different [`DerivableCurve`] implementatons:
    /// some curves can sign arbitrary byte slices, while others might expect
    /// prehashed messages of a specific size.
    type Message: ?Sized;

    /// Signature of the curve
    type Signature;

    /// Verify the signature over the message for given public key
    fn verify(
        public_key: &Self::PublicKey,
        msg: &Self::Message,
        signature: &Self::Signature,
    ) -> bool;
}

/// Derivable public key.
/// See [`DerivableCurve`].
pub trait DerivablePublicKey<C: DerivableCurve + ?Sized> {
    /// Derive public key with given [path](DerivableCurve::Path).
    #[must_use]
    fn derive(&self, path: &C::Path) -> Self;
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
pub trait DeriveSigner<C: DerivableCurve + ?Sized> {
    /// Get root public key of the signer
    fn public_key(&self) -> C::PublicKey;

    /// Sign given message with a secret key **internally** derived for given
    /// [`path`](DerivableCurve::Path).
    ///
    /// NOTE: the returned signatures are non-deterministic, i.e.
    /// implementations MAY return different signatures for the same
    /// `path` and `msg`.
    fn derive_sign(&self, path: &C::Path, msg: &C::Message) -> C::Signature;

    /// Helper method to derive public key from [root](Self::public_key)
    /// for given [path](DerivableCurve::Path)
    fn derive_public_key(&self, path: &C::Path) -> C::PublicKey {
        self.public_key().derive(path)
    }
}

#[cfg(all(test, feature = "signing"))]
mod tests {
    use std::fmt::Debug;

    use super::*;

    #[track_caller]
    pub fn assert_roundtrip<C, S>(
        root_sk: S,
        path: &C::Path,
        msg: &C::Message,
    ) -> (C::PublicKey, C::Signature)
    where
        C: DerivableCurve,
        S: DeriveSigner<C>,
    {
        let derived_pk = root_sk.public_key().derive(path);
        let signature = root_sk.derive_sign(path, msg);

        assert!(C::verify(&derived_pk, msg, &signature), "invalid signature");

        (derived_pk, signature)
    }

    #[track_caller]
    pub fn assert_roundtrip_expected<C, S>(
        root_sk: S,
        path: &C::Path,
        msg: &C::Message,
        expected_derived_pk: &C::PublicKey,
    ) -> C::Signature
    where
        C: DerivableCurve,
        C::PublicKey: PartialEq + Debug,
        S: DeriveSigner<C>,
    {
        let (derived_pk, signature) = assert_roundtrip(root_sk, path, msg);
        assert_eq!(
            &derived_pk, expected_derived_pk,
            "derived public key has changed"
        );
        signature
    }
}
