use defuse_outlayer_host::crypto::secp256k1::Secp256k1Host;
use defuse_outlayer_sys::host::outlayer;
use defuse_outlayer_worker_host::crypto::secp256k1::WorkerSecp256k1Host;
use impl_tools::autoimpl;

#[derive(Debug, Default)]
#[autoimpl(Deref using self.0)]
pub struct Secp256k1HostState(WorkerSecp256k1Host);

impl outlayer::crypto::secp256k1::Host for Secp256k1HostState {
    fn derive_public_key(&mut self, path: String) -> Vec<u8> {
        self.secp256k1_derive_public_key(path.as_str()).to_vec()
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
        self.secp256k1_sign(path.as_str(), msg.as_slice()).to_vec()
    }
}
