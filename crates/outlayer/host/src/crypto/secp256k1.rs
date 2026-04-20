use std::{rc::Rc, sync::Arc};

use impl_tools::autoimpl;

// TODO: use defuse crypto?
pub type Secp256k1PublicKey = [u8; 64];
pub type Secp256k1Signature = [u8; 65];

#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
pub trait Secp256k1Host {
    fn secp256k1_derive_public_key(&self, path: impl AsRef<str>) -> Secp256k1PublicKey;
    fn secp256k1_sign(&self, path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Secp256k1Signature;
}
