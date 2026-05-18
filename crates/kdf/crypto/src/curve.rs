/// An ellipitc curve.
pub trait Curve: 'static {
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
