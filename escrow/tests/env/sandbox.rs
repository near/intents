use std::sync::Arc;

use near_api::{NetworkConfig, RPCEndpoint, Signer};
use near_sandbox::GenesisAccount;
use near_sdk::AccountId;

pub struct Sandbox {
    network_config: NetworkConfig,

    root_id: AccountId,
    root_signer: Arc<Signer>,

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
        let root_id: AccountId = root.account_id;
        let root_signer: Arc<Signer> =
            Signer::new(Signer::from_secret_key(root.private_key.parse().unwrap())).unwrap();

        Self {
            network_config,
            root_id,
            root_signer,
            sandbox,
        }
    }

    pub const fn root_id(&self) -> &AccountId {
        &self.root_id
    }

    pub fn root_signer(&self) -> Arc<Signer> {
        self.root_signer.clone()
    }

    pub fn network_config(&self) -> &NetworkConfig {
        &self.network_config
    }

    pub fn subaccount(&self, name: impl AsRef<str>) -> AccountId {
        format!("{}.{}", name.as_ref(), self.root_id())
            .parse()
            .unwrap()
    }
}
