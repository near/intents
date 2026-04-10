use crate::{CryptoHost, Curve, DefaultHost};

// TODO: use defuse crypto?
pub struct Secp256k1Curve;

impl Curve for Secp256k1Curve {
    type PublicKey = [u8; 64];
    type Signature = [u8; 65];
}

impl CryptoHost<Secp256k1Curve> for DefaultHost {
    fn get_project_public_key(&self) -> <Secp256k1Curve as Curve>::PublicKey {
        unimplemented!("get_project_public_key is not implemented for DefaultHost");
    }

    fn derive_public_key(&self, _path: impl AsRef<str>) -> <Secp256k1Curve as Curve>::PublicKey {
        unimplemented!("derive_public_key is not implemented for DefaultHost");
    }

    fn sign(
        &self,
        _path: impl AsRef<str>,
        _msg: impl AsRef<[u8]>,
    ) -> <Secp256k1Curve as Curve>::Signature {
        unimplemented!("sign is not implemented for DefaultHost");
    }
}
