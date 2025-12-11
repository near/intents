#[cfg(feature = "contract")]
mod contract;
mod message;
pub mod state;

use near_plugins::AccessControlRole;
use near_sdk::{ext_contract, near};

pub use message::*;
pub use state::{ProxyConfig, RolesConfig};

#[cfg(feature = "test-utils")]
pub mod ext;

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

use defuse_transfer_auth::TransferAuthContext;
use near_sdk::CryptoHash;

#[ext_contract(ext_escrow_proxy)]
pub trait EscrowProxy {
    fn config(&self) -> &ProxyConfig;
    fn context_hash(&self, context: TransferAuthContext<'static>) -> CryptoHash;
}

// fix JsonSchema macro bug
#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
use near_sdk::serde;
