mod account;
pub mod extensions;
pub mod tx;

pub use account::{Account, SigningAccount};

pub use anyhow;
pub use near_api as api;

use near_api::{NetworkConfig, RPCEndpoint};
use near_sandbox::{FetchData, GenesisAccount};
use near_sdk::{AccountIdRef, NearToken};

use crate::extensions::account::ParentAccountExt;

pub struct Sandbox {
    root: SigningAccount,

    #[allow(dead_code)] // keep ownership
    sandbox: near_sandbox::Sandbox,
}

impl Sandbox {
    pub async fn new() -> Self {
        let sandbox = near_sandbox::Sandbox::start_sandbox().await.unwrap();

        let network_config = NetworkConfig {
            network_name: "sandbox".to_string(),
            rpc_endpoints: vec![RPCEndpoint::new(sandbox.rpc_addr.parse().unwrap())],
            ..NetworkConfig::testnet()
        };

        let root = GenesisAccount::default();

        Self {
            root: SigningAccount::new(
                Account::new(root.account_id, network_config),
                root.private_key.parse().unwrap(),
            ),
            sandbox,
        }
    }

    pub const fn root(&self) -> &SigningAccount {
        &self.root
    }

    // TODO: to trait
    pub async fn create_account(&self, name: &str) -> anyhow::Result<SigningAccount> {
        self.root
            .create_subaccount(name, NearToken::from_near(10))
            .await
    }

    pub async fn import_contract(
        &self,
        account_id: &AccountIdRef,
        rpc: impl AsRef<str>,
    ) -> anyhow::Result<()> {
        self.sandbox
            .patch_state(account_id.into())
            .fetch_from(rpc, FetchData::new().code())
            .await?;

        Ok(())
    }
}
