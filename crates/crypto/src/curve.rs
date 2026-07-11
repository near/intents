/// An ellipitc curve.
pub trait Curve: 'static {
    /// Public key of the curve
    type PublicKey;

    /// Signature of the curve
    type Signature;

    /// Verify the signature over the message for given public key
    // TODO: docs maybe prehash
    fn verify(public_key: &Self::PublicKey, msg: &[u8], signature: &Self::Signature) -> bool;
}

// TODO: feature "signing"
pub trait Signer<C: Curve> {
    type Error;

    fn public_key(&self) -> C::PublicKey;

    fn sign(&self, msg: &[u8]) -> Result<C::Signature, Self::Error>;
}

/// A recoverable [curve](Curve).
pub trait RecoverableCurve: Curve {
    /// An additional information required to [recover](Self::recover)
    /// the public key.
    type RecoveryId;

    /// Try to recover [public key](Curve::PublicKey) which signed given
    /// message and produced given signature along with a
    /// [recovery id](Self::RecoveryId)
    fn recover(
        msg: &[u8],
        signature: &Self::Signature,
        recovery_id: Self::RecoveryId,
    ) -> Option<Self::PublicKey>;
}

pub trait RecoverableSigner<C: RecoverableCurve>: Signer<C> {
    fn sign_recoverable(&self, msg: &[u8]) -> Result<(C::Signature, C::RecoveryId), Self::Error>;
}
