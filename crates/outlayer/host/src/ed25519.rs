// TODO: use defuse crypto?
pub type Ed25519PublicKey = [u8; 32];
pub type Ed25519Signature = [u8; 64];

pub trait Ed25519World {
    fn ed25519_derive_public_key(&self, path: impl AsRef<str>) -> Ed25519PublicKey;
    fn ed25519_sign(&self, path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Ed25519Signature;
}

pub struct Ed25519Host;

impl Ed25519World for Ed25519Host {
    fn ed25519_derive_public_key(&self, _path: impl AsRef<str>) -> Ed25519PublicKey {
        unimplemented!("ed25519_derive_public_key is not implemented for DefaultHost");
    }

    fn ed25519_sign(&self, _path: impl AsRef<str>, _msg: impl AsRef<[u8]>) -> Ed25519Signature {
        unimplemented!("ed25519_sign is not implemented for DefaultHost");
    }
}
