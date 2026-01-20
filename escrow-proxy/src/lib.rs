#[cfg(feature = "contract")]
mod contract;
mod message;
pub mod state;

use near_sdk::{Gas, ext_contract};

pub use message::*;
pub use state::ProxyConfig;

/// Minimum gas required for proxy `mt_on_transfer`.
pub const MT_ON_TRANSFER_GAS: Gas = Gas::from_tgas(70);

/// Minimum gas required for proxy `ft_on_transfer`.
pub const FT_ON_TRANSFER_GAS: Gas = Gas::from_tgas(70);

use defuse_oneshot_condvar::CondVarContext;
use near_sdk::{AccountId, CryptoHash, json_types::U128};

#[ext_contract(ext_escrow_proxy)]
pub trait EscrowProxy {
    fn config(&self) -> &ProxyConfig;
    fn context_hash(&self, context: CondVarContext<'static>) -> CryptoHash;
    fn oneshot_address(
        &self,
        taker_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> AccountId;
}
