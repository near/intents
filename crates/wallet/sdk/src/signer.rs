use std::sync::Arc;

use async_trait::async_trait;
use defuse_wallet::{RequestMessage, SignatureSchema};
use impl_tools::autoimpl;

pub type Proof = String;

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[autoimpl(for<T: ?Sized + trait> &T, &mut T, Box<T>, Arc<T>)]
pub trait Signer<S: SignatureSchema> {
    type Error;

    fn public_key(&self) -> S::PublicKey;
    async fn sign(&self, msg: &RequestMessage) -> Result<Proof, Self::Error>;
}
