/// An ellipitc curve.
pub trait Curve: 'static {
    // /// A path to derive both [public](DerivablePublicKey::derive) and
    // /// [signing](DeriveSigner::derive_sign) keys for.
    // /// Typically, it should be an output of a cryptographic hash function.
    // type Path: ?Sized;

    /// Public key of the curve
    type PublicKey;

    /// Message for signing.
    ///
    /// This type can vary between different curve implementations:
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
