// TODO: use defuse crypto?
pub type Secp256k1PublicKey = [u8; 64];
pub type Secp256k1Signature = [u8; 65];

pub trait Secp256k1World {
    fn secp256k1_derive_public_key(&self, path: impl AsRef<str>) -> Secp256k1PublicKey;
    fn secp256k1_sign(&self, path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Secp256k1Signature;
}

pub struct Secp256k1Host;

impl Secp256k1World for Secp256k1Host {
    fn secp256k1_derive_public_key(&self, _path: impl AsRef<str>) -> Secp256k1PublicKey {
        unimplemented!("secp256k1_derive_public_key is not implemented for DefaultHost");
    }

    fn secp256k1_sign(&self, _path: impl AsRef<str>, _msg: impl AsRef<[u8]>) -> Secp256k1Signature {
        unimplemented!("secp256k1_sign is not implemented for DefaultHost");
    }
}
