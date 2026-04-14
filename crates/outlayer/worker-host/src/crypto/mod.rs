pub mod ed25519;
pub mod secp256k1;

pub struct CryptoHost {
    ed25519: ed25519::WorkerEd25519Host,
    secp256k1: secp256k1::WorkerSecp256k1Host,
}
