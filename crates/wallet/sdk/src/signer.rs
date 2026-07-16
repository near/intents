use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use defuse_wallet::{AuthMessage, RequestMessage, SignatureSchema};
use impl_tools::autoimpl;

/// A proof for [`w_execute_signed(msg, proof)`](defuse_wallet::contract::Wallet::w_execute_signed)
pub type Proof = String;

/// A signer that can sign [`RequestMessage`] according to specific
/// [`SignatureSchema`].
///
/// For usage, see [`Wallet`](crate::Wallet).
#[trait_variant::make(Send)]
#[autoimpl(for<T: ?Sized + trait> &T, &mut T, Box<T>, Arc<T>)]
pub trait WalletSigner<S: SignatureSchema>: Sync {
    /// Signature error
    type Error: Debug + Display;

    /// Returns public key of the signer.
    fn public_key(&self) -> S::PublicKey;

    /// Sign [`RequestMessage`] according to [`SignatureSchema`]
    /// and return a proof serialized to string ready to be submitted to
    /// [`w_execute_signed(msg, proof)`](defuse_wallet::contract::Wallet::w_execute_signed) contract method
    async fn sign_request_msg(&self, msg: &RequestMessage) -> Result<Proof, Self::Error>;

    /// Sign [`AuthMessage`] (NEP-641) according to [`SignatureSchema`]
    /// and return a proof serialized to string ready to be wrapped in a
    /// [`SignedAuthMessage`](defuse_wallet::SignedAuthMessage) and resolved via
    /// [`w_resolve_auth()`](defuse_wallet::contract::Wallet::w_resolve_auth)
    /// contract method
    async fn sign_auth_msg(&self, msg: &AuthMessage) -> Result<Proof, Self::Error>;
}
