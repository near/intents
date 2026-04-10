use defuse_outlayer_host::{secp256k1::Secp256k1Curve, CryptoHost, Curve};
use outlayer::host::secp256k1;

use crate::SysHost;

wit_bindgen::generate!({
    path: "../wit",
    world: "secp256k1-world",
});

impl CryptoHost<Secp256k1Curve> for SysHost {
    fn get_project_public_key(&self) -> <Secp256k1Curve as Curve>::PublicKey {
        secp256k1::get_project_public_key()
            .expect("failed to get project public key")
            .bytes
            .try_into()
            .expect("public key must be 32 bytes")
    }

    fn derive_public_key(&self, path: impl AsRef<str>) -> <Secp256k1Curve as Curve>::PublicKey {
        secp256k1::derive_public_key(path.as_ref())
            .expect("failed to derive public key")
            .bytes
            .try_into()
            .expect("public key must be 32 bytes")
    }

    fn sign(
        &self,
        path: impl AsRef<str>,
        msg: impl AsRef<[u8]>,
    ) -> <Secp256k1Curve as Curve>::Signature {
        secp256k1::sign(path.as_ref(), msg.as_ref())
            .expect("failed to sign message")
            .bytes
            .try_into()
            .expect("signature must be 64 bytes")
    }
}
