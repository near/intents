use crate::Algorithm;

/// [COSE EdDSA (-8) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// ed25519 curve
pub struct Ed25519;

impl Algorithm for Ed25519 {
    type Curve = defuse_kdf_crypto::Ed25519;

    fn derive(msg: impl AsRef<[u8]>) -> impl AsRef<[u8]> {
        // sign via ed25519 as-is
        msg
    }
}
