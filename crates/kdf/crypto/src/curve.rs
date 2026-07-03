// TODO: curve naming

/// An ellipitc curve.
pub trait Curve: 'static {
    /// Public key of the curve
    type PublicKey;

    // /// Message for signing.
    // ///
    // /// This type can vary between different curve implementations:
    // /// some curves can sign arbitrary byte slices, while others might expect
    // /// prehashed messages of a specific size.
    // type Message: ?Sized;

    /// Signature of the curve
    type Signature;
}

pub trait VerifiableCurve<M>: Curve {
    /// Verify the signature over the message for given public key
    fn verify(public_key: &Self::PublicKey, msg: M, signature: &Self::Signature) -> bool;
}

/// A recoverable [curve](Curve).
pub trait RecoverableCurve<M>: VerifiableCurve<M> {
    /// An additional information required to [recover](Self::recover)
    /// the public key.
    type RecoveryId;

    /// Try to recover [public key](Curve::PublicKey) which signed given
    /// message and produced given signature along with a
    /// [recovery id](Self::RecoveryId)
    fn recover(
        msg: M,
        signature: &Self::Signature,
        recovery_id: Self::RecoveryId,
    ) -> Option<Self::PublicKey>;

    // TODO: fn trial_recover()?
}
