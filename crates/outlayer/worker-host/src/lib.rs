pub mod crypto;

use hkdf::Hkdf;
use near_account_id::AccountId;
use sha2::Sha256;

use self::crypto::CryptoHost;

pub struct WorkerHost {
    crypto: CryptoHost,
}

impl WorkerHost {
    /// Constructs host from the initial key material
    pub fn from_ikm(ikm: &[u8], info: &[u8]) -> Self {
        let hk = Hkdf::<Sha256>::new(None, ikm);
        let mut seed = [0u8; 32];
        hk.expand(info, &mut seed)
            .expect("32 bytes is a valid length for Sha256 to output");
        Self::from_seed(seed)
    }

    // TODO: +app_id
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            crypto: CryptoHost::from_seed(seed),
        }
    }
}

// TODO: resource in WIT?
pub enum AppId {
    Near(AccountId),
}
