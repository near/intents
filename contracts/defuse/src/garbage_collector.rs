use defuse_core::Nonce;
use defuse_serde_utils::base64::AsBase64;
use near_sdk::{AccountId, ext_contract};

#[ext_contract(ext_garbage_collector)]
#[allow(clippy::module_name_repetitions)]
pub trait GarbageCollector {
    /// Clears all expired nonces for given accounts by its prefix.
    /// Omitting any errors, e.g. if account doesn't exist or nonces are not expired.
    /// NOTE: MUST attach 1 yⓃ for security purposes.
    fn cleanup_nonces(&mut self, nonces: Vec<(AccountId, Vec<AsBase64<Nonce>>)>);
}
