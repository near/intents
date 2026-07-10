use crate::RequestMessage;

/// Signature schema used by wallet contract variant.
pub trait SignatureSchema {
    /// Public key stored in the contract's state.
    type PublicKey;

    /// Verify given proof over the request message in respect to the public
    /// key and return whether verification passed.
    ///
    /// Used by the `w_execute_signed(msg, proof)` contract method.
    fn verify(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool;
}
