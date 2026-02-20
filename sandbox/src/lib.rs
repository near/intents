mod account;
pub mod extensions;
pub mod helpers;
pub mod tx;

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::OnceCell;

pub use account::{Account, SigningAccount};
pub use extensions::{
    ft::{FtExt, FtViewExt},
    mt::{MtExt, MtViewExt},
    mt_receiver::MtReceiverStubExt,
    storage_management::{StorageManagementExt, StorageViewExt},
    wnear::{WNearDeployerExt, WNearExt},
};
pub use helpers::*;
pub use tx::{FnCallBuilder, TxBuilder};

pub use anyhow;
use impl_tools::autoimpl;
pub use near_api as api;
// NOTE: that is not ok - errors should be separated to another mod
pub use near_openapi_types as openapi_types;
pub use near_sandbox;

use near_api::{NetworkConfig, RPCEndpoint, Signer, signer::generate_secret_key};
use near_sandbox::{GenesisAccount, SandboxConfig};
use near_sdk::{AccountId, AccountIdRef, NearToken};
use rstest::fixture;
use tracing::instrument;

#[autoimpl(Deref using self.root)]
pub struct Sandbox {
    root: SigningAccount,
    sandbox: Arc<near_sandbox::Sandbox>,
}

impl Sandbox {
    pub async fn new(root_id: impl Into<AccountId>) -> Self {
        let root_id = root_id.into();

        // FIXME: why does test.ner exist in genesis cfg????
        let root_secret_key = generate_secret_key().unwrap();

        let root_genesis = GenesisAccount {
            account_id: root_id.clone(),
            public_key: root_secret_key.public_key().to_string(),
            private_key: root_secret_key.to_string(),
            // half of total supply
            balance: NearToken::MAX.saturating_div(2),
        };

        let sandbox = near_sandbox::Sandbox::start_sandbox_with_config(SandboxConfig {
            additional_accounts: vec![root_genesis],
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
            Account::new(root_id, network_config),
            Signer::from_secret_key(root_secret_key).unwrap(),
        );

        Self {
            root,
            sandbox: sandbox.into(),
        }
    }

    pub const fn root(&self) -> &SigningAccount {
        &self.root
    }

    pub fn account(&self, account_id: impl Into<AccountId>) -> Account {
        Account::new(account_id, self.root().network_config().clone())
    }

    pub fn sandbox(&self) -> &near_sandbox::Sandbox {
        self.sandbox.as_ref()
    }

    pub async fn fast_forward(&self, blocks: u64) {
        self.sandbox.fast_forward(blocks).await.unwrap();
    }
}

/// Shared sandbox instance for test fixtures.
/// Using `OnceCell<Mutex<Option<...>>>` allows async init and taking ownership in atexit.
static SHARED_SANDBOX: OnceCell<Mutex<Option<Sandbox>>> = OnceCell::const_new();

extern "C" fn cleanup_sandbox() {
    if let Some(mutex) = SHARED_SANDBOX.get() {
        if let Ok(mut guard) = mutex.lock() {
            drop(guard.take());
        }
    }
}

pub const ROOT_PK_POOL_SIZE: usize = 10;

#[fixture]
#[instrument]
pub async fn sandbox(#[default(NearToken::from_near(100_000))] amount: NearToken) -> Sandbox {
    const SHARED_ROOT: &AccountIdRef = AccountIdRef::new_or_panic("test");
    static SUB_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let mutex = SHARED_SANDBOX
        .get_or_init(|| async {
            unsafe {
                libc::atexit(cleanup_sandbox);
            }
            Mutex::new(Some(Sandbox::new(SHARED_ROOT).await))
        })
        .await;

    let (sandbox_arc, root_account) = mutex
        .lock()
        .unwrap()
        .as_ref()
        .map(|shared| (shared.sandbox.clone(), shared.root.clone()))
        .unwrap();

    let child_root = root_account
        .generate_subaccount_highload(
            SUB_COUNTER.fetch_add(1, Ordering::Relaxed).to_string(),
            ROOT_PK_POOL_SIZE,
            amount,
        )
        .await
        .unwrap();

    Sandbox {
        root: child_root,
        sandbox: sandbox_arc,
    }
}
