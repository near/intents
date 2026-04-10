use crate::DefaultHost;

// TODO: use defuse crypto?
pub type Secp256k1PublicKey = [u8; 64];
pub type Secp256k1Signature = [u8; 65];

pub trait Secp256k1Host {
    fn secp256k1_get_project_public_key() -> Secp256k1PublicKey;
    fn secp256k1_derive_public_key(_path: impl AsRef<str>) -> Secp256k1PublicKey;
    fn secp256k1_sign(_path: impl AsRef<str>, _msg: impl AsRef<[u8]>) -> Secp256k1Signature;
}

impl Secp256k1Host for DefaultHost {
    fn secp256k1_get_project_public_key() -> Secp256k1PublicKey {
        unimplemented!("secp256k1_get_project_public_key is not implemented for DefaultHost");
    }

    fn secp256k1_derive_public_key(_path: impl AsRef<str>) -> Secp256k1PublicKey {
        unimplemented!("secp256k1_derive_public_key is not implemented for DefaultHost");
    }

    fn secp256k1_sign(_path: impl AsRef<str>, _msg: impl AsRef<[u8]>) -> Secp256k1Signature {
        unimplemented!("secp256k1_sign is not implemented for DefaultHost");
    }
}
