#[cfg(feature = "contract")]
mod contract;
mod message;
pub mod state;

use near_plugins::AccessControlRole;
use near_sdk::{ext_contract, near};

pub use message::*;
pub use state::{ProxyConfig, RolesConfig};

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
