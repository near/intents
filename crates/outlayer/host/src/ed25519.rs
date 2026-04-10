use crate::{CryptoHost, Curve, DefaultHost};

// TODO: use defuse crypto?
pub struct Ed25519Curve;

impl Curve for Ed25519Curve {
    type PublicKey = [u8; 32];
    type Signature = [u8; 64];
}

impl CryptoHost<Ed25519Curve> for DefaultHost {
    fn get_project_public_key(&self) -> <Ed25519Curve as Curve>::PublicKey {
        unimplemented!("get_project_public_key is not implemented for DefaultHost");
    }

    fn derive_public_key(&self, _path: impl AsRef<str>) -> <Ed25519Curve as Curve>::PublicKey {
        unimplemented!("derive_public_key is not implemented for DefaultHost");
    }

    fn sign(
        &self,
        _path: impl AsRef<str>,
        _msg: impl AsRef<[u8]>,
    ) -> <Ed25519Curve as Curve>::Signature {
        unimplemented!("sign is not implemented for DefaultHost");
    }
}
