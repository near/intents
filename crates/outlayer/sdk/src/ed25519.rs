use crate::SysHost;
use defuse_outlayer_host::{ed25519::Ed25519Curve, CryptoHost, Curve};
use outlayer::host::ed25519;

// TODO: may be make symlink?
wit_bindgen::generate!({
    path: "../wit",
    world: "ed25519-world",
});

impl CryptoHost<Ed25519Curve> for SysHost {
    fn get_project_public_key(&self) -> <Ed25519Curve as Curve>::PublicKey {
        ed25519::get_project_public_key()
            .expect("failed to get project public key")
            .bytes
            .try_into()
            .expect("public key must be 32 bytes")
    }

    fn derive_public_key(&self, path: impl AsRef<str>) -> <Ed25519Curve as Curve>::PublicKey {
        ed25519::derive_public_key(path.as_ref())
            .expect("failed to derive public key")
            .bytes
            .try_into()
            .expect("public key must be 32 bytes")
    }

    fn sign(
        &self,
        path: impl AsRef<str>,
        msg: impl AsRef<[u8]>,
    ) -> <Ed25519Curve as Curve>::Signature {
        ed25519::sign(path.as_ref(), msg.as_ref())
            .expect("failed to sign message")
            .bytes
            .try_into()
            .expect("signature must be 64 bytes")
    }
}
