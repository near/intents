use outlayer::host::secp256k1;

pub use defuse_outlayer_host::secp256k1::{Secp256k1Host, Secp256k1PublicKey, Secp256k1Signature};

use crate::SysHost;

wit_bindgen::generate!({
    path: "../wit",
    world: "secp256k1-world",
});

impl Secp256k1Host for SysHost {
    fn secp256k1_get_project_public_key() -> Secp256k1PublicKey {
        secp256k1::get_project_public_key()
            .try_into()
            .expect("public key must be 64 bytes")
    }

    fn secp256k1_derive_public_key(path: impl AsRef<str>) -> Secp256k1PublicKey {
        secp256k1::derive_public_key(path.as_ref())
            .try_into()
            .expect("public key must be 64 bytes")
    }

    fn secp256k1_sign(path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Secp256k1Signature {
        secp256k1::sign(path.as_ref(), msg.as_ref())
            .try_into()
            .expect("signature must be 65 bytes")
    }
}
