#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "secp256k1")]
pub mod secp256k1;

use defuse_kdf_crypto::Curve;

/// A [curve](Curve) with **non-hardened** public key derivation capabilities,
/// i.e. when child public key can be [derived](DerivableCurve::derive_public_key)
/// from the [master](crate::DeriveSigner::public_key) without knowing any
/// secret.
///
/// The derivation is **non-hierarchical** (i.e. "additive", or "plain"):
/// derived keys **do not** form a tree-like structure. Instead, child public
/// keys are all derived from a single master key and can be considered as
/// "peers" to each other.
///
/// For signing, see [`DeriveSigner`](crate::DeriveSigner).
pub trait DerivableCurve: Curve {
    /// A tweak to derive both [public](DerivableCurve::derive_public_key) and
    /// [signing](crate::DeriveSigner::derive_sign) keys for.
    ///
    /// Typically, it should be derived from an output of a cryptographic
    /// hash function. See [`DerivationSchema`](crate::DerivationSchema).
    type Tweak;

    /// Derive public key from [master](crate::DeriveSigner::public_key)
    /// for given [tweak](DerivableCurve::Tweak)
    #[must_use]
    fn derive_public_key(master_pk: &Self::PublicKey, tweak: &Self::Tweak) -> Self::PublicKey;
}
