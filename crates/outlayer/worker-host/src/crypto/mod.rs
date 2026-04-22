pub mod ed25519;
pub mod secp256k1;

use digest_io::IoWrapper;
use sha3::{Digest, Sha3_256};

use crate::WorkerHost;

impl WorkerHost {
    pub(crate) fn derive_tweak(&self, path: impl AsRef<[u8]>) -> [u8; 32] {
        let mut hasher = IoWrapper(Sha3_256::new());

        borsh::to_writer(&mut hasher, &(&self.app_id, path.as_ref()))
            .expect("IoWrapper<Sha3_256> is infallible");

        hasher.0.finalize().into()
    }
}
