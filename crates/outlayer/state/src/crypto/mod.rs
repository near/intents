mod ed25519;
mod secp256k1;

use defuse_outlayer_worker_host::crypto::{
    ed25519::WorkerEd25519Host, secp256k1::WorkerSecp256k1Host,
};

#[derive(Debug, Default)]
pub struct CryptoHostState {
    ed25519: WorkerEd25519Host,
    secp256k1: WorkerSecp256k1Host,
}
