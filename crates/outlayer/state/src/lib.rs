pub mod crypto;

pub struct HostState {
    crypto: crypto::CryptoHostState,
}

impl HostState {
    pub const fn new(crypto: crypto::CryptoHostState) -> Self {
        Self { crypto }
    }
}
