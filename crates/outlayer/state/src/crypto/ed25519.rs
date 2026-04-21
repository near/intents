use defuse_outlayer_host::crypto::ed25519::Ed25519Host;
use defuse_outlayer_host_functions::outlayer;

use crate::HostState;

impl outlayer::crypto::ed25519::Host for HostState {
    fn derive_public_key(&mut self, path: String) -> Vec<u8> {
        self.worker
            .ed25519_derive_public_key(path.as_str())
            .to_vec()
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
        self.worker
            .ed25519_sign(path.as_str(), msg.as_slice())
            .to_vec()
    }
}
