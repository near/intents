#[cfg(feature = "crypto")]
pub mod crypto;

pub struct WorkerHost {
    #[cfg(feature = "crypto")]
    crypto: crypto::CryptoHost,
}
