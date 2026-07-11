/// Digital Signature Algorithm.
pub trait Curve: 'static {
    /// Public key
    type PublicKey;

    /// Signature
    type Signature;

    /// Verify the signature over the message for given public key
    ///
    /// NOTE: implementations MAY require `msg` to be prehash (i.e. output
    /// of cryptographic hash function) of a fixed length and reject
    /// the signature otherwise. Check corresponding docs before using.
    fn verify(public_key: &Self::PublicKey, msg: &[u8], signature: &Self::Signature) -> bool;
}

/// A recoverable [curve](Curve).
pub trait RecoverableCurve: Curve {
    /// An additional information required to [recover](Self::recover)
    /// the public key.
    type RecoveryId;

    /// Try to recover [public key](Curve::PublicKey) which signed given
    /// message and produced given signature along with a
    /// [recovery id](Self::RecoveryId)
    ///
    /// NOTE: implementations MAY require `msg` to be prehash (i.e. output
    /// of cryptographic hash function) of a fixed length and reject
    /// the signature otherwise. Check corresponding docs before using.
    fn recover(
        msg: &[u8],
        signature: &Self::Signature,
        recovery_id: Self::RecoveryId,
    ) -> Option<Self::PublicKey>;
}
