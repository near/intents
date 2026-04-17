use defuse_outlayer_sys::host;

pub mod crypto;

pub struct HostState {
    crypto: crypto::CryptoHostState,
}

impl HostState {
    pub fn new(crypto: crypto::CryptoHostState) -> Self {
        Self { crypto }
    }
}

impl host::Host for HostState {
    type Ed25519 = crypto::Ed25519HostState;
    type Secp256k1 = crypto::Secp256k1HostState;

    fn ed25519(&mut self) -> &mut Self::Ed25519 {
        self.crypto.ed25519()
    }

    fn secp256k1(&mut self) -> &mut Self::Secp256k1 {
        self.crypto.secp256k1()
    }
}
