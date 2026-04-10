#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "secp256k1")]
pub mod secp256k1;

pub trait Curve {
    type PublicKey;
    type Signature;
}

pub trait CryptoHost<C: Curve> {
    fn get_project_public_key(&self) -> C::PublicKey;
    fn derive_public_key(&self, path: impl AsRef<str>) -> C::PublicKey;
    fn sign(&self, path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> C::Signature;
}

pub struct DefaultHost;
