mod ed25519;
mod secp256k1;

pub use self::{ed25519::Ed25519HostState, secp256k1::Secp256k1HostState};

#[derive(Debug, Default)]
pub struct CryptoHostState {
    ed25519: Ed25519HostState,
    secp256k1: Secp256k1HostState,
}

impl CryptoHostState {
    pub const fn ed25519(&mut self) -> &mut Ed25519HostState {
        &mut self.ed25519
    }

    pub const fn secp256k1(&mut self) -> &mut Secp256k1HostState {
        &mut self.secp256k1
    }
}
