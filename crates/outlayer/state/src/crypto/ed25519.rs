use defuse_outlayer_host::crypto::ed25519::Ed25519Host;
use defuse_outlayer_sys::host::outlayer;
use defuse_outlayer_worker_host::crypto::ed25519::WorkerEd25519Host;
use impl_tools::autoimpl;

#[autoimpl(Deref using self.0)]
pub struct Ed25519HostState(WorkerEd25519Host);

impl Ed25519HostState {
    pub fn new() -> Self {
        Self(WorkerEd25519Host)
    }
}

impl outlayer::crypto::ed25519::Host for Ed25519HostState {
    fn derive_public_key(&mut self, path: String) -> Vec<u8> {
        self.ed25519_derive_public_key(path.as_str()).to_vec()
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
        self.ed25519_sign(path.as_str(), msg.as_slice()).to_vec()
    }
}
