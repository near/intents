#![allow(async_fn_in_trait)]

mod account;
mod tx;

pub use self::{account::*, tx::*};
pub use near_api as api;

use near_api::{NetworkConfig, RPCEndpoint, Signer};
use near_sandbox::GenesisAccount;

pub struct Sandbox {
    root: SigningAccount,

    #[allow(dead_code)] // keep ownership
    sandbox: near_sandbox::Sandbox,
}

impl Sandbox {
    pub async fn new() -> Self {
        let sandbox = near_sandbox::Sandbox::start_sandbox_with_version("2.10-release")
            .await
            .unwrap();

        let network_config = NetworkConfig {
            network_name: "sandbox".to_string(),
            rpc_endpoints: vec![RPCEndpoint::new(sandbox.rpc_addr.parse().unwrap())],
            ..NetworkConfig::testnet()
        };

        let root = GenesisAccount::default();
        let signer = Signer::from_secret_key(root.private_key.parse().unwrap()).unwrap();
        Self {
            root: SigningAccount::new(Account::new(root.account_id, network_config), signer),
            sandbox,
        }
    }

    pub const fn root(&self) -> &SigningAccount {
        &self.root
    }
}
