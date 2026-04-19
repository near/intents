use std::{rc::Rc, sync::Arc};

use impl_tools::autoimpl;

// TODO: use defuse crypto?
pub type Ed25519PublicKey = [u8; 32];
pub type Ed25519Signature = [u8; 64];

#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
pub trait Ed25519Host {
    fn ed25519_derive_public_key(&self, path: impl AsRef<str>) -> Ed25519PublicKey;
    fn ed25519_sign(&self, path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Ed25519Signature;
}
