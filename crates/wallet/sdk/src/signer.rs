use std::sync::Arc;

use defuse_wallet::{RequestMessage, SignatureSchema};
use impl_tools::autoimpl;

/// A proof for [`w_execute_signed(msg, proof)`](defuse_wallet::contract::Wallet::w_execute_signed)
pub type Proof = String;

/// A signer that can sign [`RequestMessage`] according to specific
/// [`SignatureSchema`].
#[cfg_attr(
    not(target_family = "wasm"),
    trait_variant::make(Send),
    autoimpl(for<T: ?Sized + trait + Sync> &T, Arc<T>)
)]
#[cfg_attr(
    target_family = "wasm",
    autoimpl(for<T: ?Sized + trait> &T, Arc<T>)
)]
#[autoimpl(for<T: ?Sized + trait> &mut T, Box<T>)]
pub trait WalletSigner<S: SignatureSchema> {
    /// Signature error
    type Error;

    /// Returns public key of the signer.
    fn public_key(&self) -> S::PublicKey;

    /// Sign [`RequestMessage`] according to [`SignatureSchema`]
    /// and return a proof serialized to string ready to be submitted to
    /// [`w_execute_signed(msg, proof)`](defuse_wallet::contract::Wallet::w_execute_signed) contract method
    async fn sign_request_msg(&self, msg: &RequestMessage) -> Result<Proof, Self::Error>;
}
