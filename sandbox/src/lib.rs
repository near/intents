mod account;
pub mod extensions;
pub mod helpers;
pub mod tx;

pub use account::{Account, SigningAccount};
pub use helpers::*;

pub use anyhow;
pub use near_api as api;
pub use near_sandbox as sandbox;

use near_api::{NetworkConfig, RPCEndpoint};
use near_sandbox::GenesisAccount;
use near_sdk::{AccountId, NearToken};

use crate::extensions::account::ParentAccountExt;

pub struct Sandbox {
    root: SigningAccount,

    sandbox: near_sandbox::Sandbox,
}

impl Sandbox {
    pub async fn new(_name: AccountId) -> Self {
        // FIX: HOW IT WORKS: why does test.ner exist in genesis cfg????
        // let root = GenesisAccount::default_with_name(name);
        // let pk = generate_secret_key().unwrap();
        // let root = GenesisAccount {
        //     account_id: name,
        //     private_key: pk.to_string(),
        //     public_key: pk.public_key().to_string(),
        //     balance: NearToken::from_near(1000),
        // };

        // let sandbox = near_sandbox::Sandbox::start_sandbox_with_config(SandboxConfig {
        //     additional_accounts: vec![root.clone()],
        //     ..SandboxConfig::default()
        // })
        // .await
        // .unwrap();

        let root = GenesisAccount::default();
        let sandbox = near_sandbox::Sandbox::start_sandbox().await.unwrap();

        let network_config = NetworkConfig {
            network_name: "sandbox".to_string(),
            rpc_endpoints: vec![RPCEndpoint::new(sandbox.rpc_addr.parse().unwrap())],
            ..NetworkConfig::testnet()
        };

        let root = SigningAccount::new(
            Account::new(root.account_id, network_config),
            root.private_key.parse().unwrap(),
        );

        Self { root, sandbox }
    }

    pub const fn root(&self) -> &SigningAccount {
        &self.root
    }

    pub const fn sandbox(&self) -> &near_sandbox::Sandbox {
        &self.sandbox
    }

    pub async fn create_account(&self, name: &str) -> anyhow::Result<SigningAccount> {
        self.root
            .create_subaccount(name, NearToken::from_near(10))
            .await
    }
}
