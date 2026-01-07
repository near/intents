#[cfg(feature = "contract")]
mod contract;
mod message;
pub mod state;

use near_plugins::AccessControlRole;
use near_sdk::{Gas, ext_contract, near};

pub use message::*;
pub use state::{ProxyConfig, RolesConfig};

/// Minimum gas required for proxy `mt_on_transfer`.
pub const MT_ON_TRANSFER_GAS: Gas = Gas::from_tgas(70);

use defuse_oneshot_condvar::CondVarContext;
use near_sdk::CryptoHash;

#[near(serializers = [json])]
#[derive(AccessControlRole, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Can upgrade the contract
    DAO,
    /// Can upgrade the contract
    Upgrader,
    /// Can call cancel on the proxy contracts
    Canceller,
}


#[ext_contract(ext_escrow_proxy)]
pub trait EscrowProxy {
    fn config(&self) -> &ProxyConfig;
    fn context_hash(&self, context: CondVarContext<'static>) -> CryptoHash;
}
