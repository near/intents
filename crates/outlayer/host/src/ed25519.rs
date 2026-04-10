use crate::DefaultHost;

// TODO: use defuse crypto?
pub type Ed25519PublicKey = [u8; 32];
pub type Ed25519Signature = [u8; 64];

pub trait Ed25519Host {
    fn ed25519_get_project_public_key() -> Ed25519PublicKey;
    fn ed25519_derive_public_key(_path: impl AsRef<str>) -> Ed25519PublicKey;
    fn ed25519_sign(_path: impl AsRef<str>, _msg: impl AsRef<[u8]>) -> Ed25519Signature;
}

impl Ed25519Host for DefaultHost {
    fn ed25519_get_project_public_key() -> Ed25519PublicKey {
        unimplemented!("ed25519_get_project_public_key is not implemented for DefaultHost");
    }

    fn ed25519_derive_public_key(_path: impl AsRef<str>) -> Ed25519PublicKey {
        unimplemented!("ed25519_derive_public_key is not implemented for DefaultHost");
    }

    fn ed25519_sign(_path: impl AsRef<str>, _msg: impl AsRef<[u8]>) -> Ed25519Signature {
        unimplemented!("ed25519_sign is not implemented for DefaultHost");
    }
}
