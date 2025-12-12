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

use near_api::{NetworkConfig, RPCEndpoint, signer::generate_secret_key};
use near_sandbox::{GenesisAccount, SandboxConfig};
use near_sdk::{AccountId, NearToken};
use rstest::fixture;
use tokio::sync::OnceCell;

static SHARED_SANDBOX: OnceCell<Sandbox> = OnceCell::const_new();

#[fixture]
pub async fn sandbox(#[default(NearToken::from_near(100_000))] amount: NearToken) -> Sandbox {
    SHARED_SANDBOX
        .get_or_init(|| Sandbox::new("test".parse().unwrap()))
        .await
        .sub_sandbox(amount)
        .await
        .unwrap()
}

pub struct Sandbox {
    root: SigningAccount,

    sub_counter: AtomicUsize,

    sandbox: Arc<near_sandbox::Sandbox>,
}

impl Sandbox {
    pub async fn new(root: AccountId) -> Self {
        // FIX: why does test.ner exist in genesis cfg????
        let root_secret_key = generate_secret_key().unwrap();

        let root = GenesisAccount {
            account_id: root,
            public_key: root_secret_key.public_key().to_string(),
            private_key: root_secret_key.to_string(),
            balance: NearToken::from_near(1_000_000_000),
        };

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
                    self.sub_counter.fetch_add(1, Ordering::Relaxed).to_string(),
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
