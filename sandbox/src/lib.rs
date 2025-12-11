mod account;
pub mod extensions;
pub mod helpers;
pub mod tx;

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub use account::{Account, SigningAccount};
pub use helpers::*;

pub use anyhow;
pub use near_api as api;
// NOTE: that is not ok - errors should be separated to another mod
pub use near_openapi_types as openapi_types;
pub use near_sandbox;

use near_api::{NetworkConfig, RPCEndpoint};
use near_sandbox::{GenesisAccount, SandboxConfig};
use near_sdk::{AccountId, NearToken};
use rstest::fixture;
use tokio::sync::OnceCell;

pub static SHARED_SANDBOX: OnceCell<Sandbox> = OnceCell::const_new();
pub const DEFAULT_ROOT_BALANCE: NearToken = NearToken::from_near(1000);

#[fixture]
pub async fn sandbox(#[default(DEFAULT_ROOT_BALANCE)] amount: NearToken) -> Sandbox {
    Sandbox::get_or_init(amount).await.unwrap()
}

pub struct Sandbox {
    root: SigningAccount,

    sub_counter: AtomicUsize,

    sandbox: Arc<near_sandbox::Sandbox>,
}

impl Sandbox {
    pub async fn get_or_init(amount: NearToken) -> anyhow::Result<Self> {
        SHARED_SANDBOX
            .get_or_init(|| Self::new("test".parse().unwrap()))
            .await
            .sub_sandbox(amount)
            .await
    }

    pub async fn new(root: AccountId) -> Self {
        // FIX: why does test.ner exist in genesis cfg????
        let root = GenesisAccount::default_with_name(root);

        let sandbox = near_sandbox::Sandbox::start_sandbox_with_config(SandboxConfig {
            additional_accounts: vec![root.clone()],
            ..SandboxConfig::default()
        })
        .await
        .unwrap();

        let network_config = NetworkConfig {
            network_name: "sandbox".to_string(),
            rpc_endpoints: vec![RPCEndpoint::new(sandbox.rpc_addr.parse().unwrap())],
            ..NetworkConfig::testnet()
        };

        let root = SigningAccount::new(
            Account::new(root.account_id, network_config),
            root.private_key.parse().unwrap(),
        );

        Self {
            root,
            sub_counter: 0.into(),
            sandbox: sandbox.into(),
        }
    }

    pub async fn sub_sandbox(&self, amount: NearToken) -> anyhow::Result<Self> {
        Ok(Self {
            root: self
                .root()
                .create_subaccount(
                    self.sub_counter.fetch_add(1, Ordering::SeqCst).to_string(),
                    amount,
                )
                .await?,
            sub_counter: 0.into(),
            sandbox: self.sandbox.clone(),
        })
    }

    pub const fn root(&self) -> &SigningAccount {
        &self.root
    }

    pub fn sandbox(&self) -> &near_sandbox::Sandbox {
        self.sandbox.as_ref()
    }
}
