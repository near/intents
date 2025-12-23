mod multi_token_receiver;

pub use multi_token_receiver::{MT_RECEIVER_STUB_WASM, MtReceiverStubAccountExt};

use std::{fs, path::Path, sync::LazyLock};

use defuse_crypto::Payload;
use defuse_deadline::Deadline;
use defuse_escrow_proxy::{ProxyConfig, RolesConfig};
use defuse_nep413::{Nep413Payload, SignedNep413Payload};
use defuse_sandbox::{
    Account, FnCallBuilder, SigningAccount,
    api::types::transaction::actions::GlobalContractDeployMode,
};
use defuse_transfer_auth::storage::{ContractStorage, StateInit as TransferAuthStateInit};
use near_sdk::{
    AccountId, Gas, GlobalContractId, NearToken,
    state_init::{StateInit, StateInitV1},
};
use serde_json::json;

// Re-export State type for convenience
pub use defuse_transfer_auth::storage::StateInit as State;

#[track_caller]
fn read_wasm(name: impl AsRef<Path>) -> Vec<u8> {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../res/")
        .join(name)
        .with_extension("wasm");
    fs::read(filename.clone()).unwrap_or_else(|_| panic!("file {filename:?} should exists"))
}

// WASM statics
pub static ESCROW_PROXY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("defuse_escrow_proxy"));
pub static TRANSFER_AUTH_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("defuse_transfer_auth"));
pub static ESCROW_SWAP_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("defuse_escrow_swap"));

// ============================================================================
// Utility Functions
// ============================================================================

/// Sign a message with Ed25519 using a raw 32-byte secret key.
/// Returns (`public_key`, `signature`) as raw byte arrays.
pub fn sign_ed25519(secret_key: &[u8; 32], message: &[u8]) -> ([u8; 32], [u8; 64]) {
    use ed25519_dalek::{Signer, SigningKey};
    let signing_key = SigningKey::from_bytes(secret_key);
    let public_key = signing_key.verifying_key().to_bytes();
    let signature = signing_key.sign(message).to_bytes();
    (public_key, signature)
}

/// Get the public key for the given secret key
pub fn public_key_from_secret(secret_key: &[u8; 32]) -> defuse_crypto::PublicKey {
    let (pk, _) = sign_ed25519(secret_key, &[]);
    defuse_crypto::PublicKey::Ed25519(pk)
}

/// Sign intents using NEP-413 standard.
/// Returns a `MultiPayload` ready to be passed to `execute_intents`.
pub fn sign_intents(
    signer_id: &AccountId,
    secret_key: &[u8; 32],
    defuse_contract_id: &AccountId,
    nonce: [u8; 32],
    intents: Vec<defuse_core::intents::Intent>,
) -> defuse_core::payload::multi::MultiPayload {
    use defuse_core::intents::DefuseIntents;
    use defuse_core::payload::multi::MultiPayload;
    use defuse_core::payload::nep413::Nep413DefuseMessage;

    let deadline = Deadline::timeout(std::time::Duration::from_secs(120));

    let nep413_message = Nep413DefuseMessage {
        signer_id: signer_id.clone(),
        deadline,
        message: DefuseIntents { intents },
    };

    let nep413_payload = Nep413Payload::new(serde_json::to_string(&nep413_message).unwrap())
        .with_recipient(defuse_contract_id)
        .with_nonce(nonce);

    let hash = nep413_payload.hash();
    let (public_key, signature) = sign_ed25519(secret_key, &hash);

    MultiPayload::Nep413(SignedNep413Payload {
        payload: nep413_payload,
        public_key,
        signature,
    })
}

/// Derive the transfer-auth instance account ID from its state
pub fn derive_transfer_auth_account_id(
    global_contract_id: &GlobalContractId,
    state: &TransferAuthStateInit,
) -> AccountId {
    let raw_state = ContractStorage::init_state(state.clone()).unwrap();
    let state_init = StateInit::V1(StateInitV1 {
        code: global_contract_id.clone(),
        data: raw_state,
    });
    state_init.derive_account_id()
}

/// Derive the escrow-swap instance account ID from its params
pub fn derive_escrow_swap_account_id(
    global_contract_id: &AccountId,
    params: &defuse_escrow_swap::Params,
) -> AccountId {
    let raw_state = defuse_escrow_swap::ContractStorage::init_state(params).unwrap();
    let state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(global_contract_id.clone()),
        data: raw_state,
    });
    state_init.derive_account_id()
}

// ============================================================================
// EscrowProxyExt
// ============================================================================

#[allow(async_fn_in_trait)]
pub trait EscrowProxyExt {
    async fn deploy_escrow_proxy(&self, roles: RolesConfig, config: ProxyConfig) -> anyhow::Result<()>;
    async fn get_escrow_proxy_config(&self) -> anyhow::Result<ProxyConfig>;
}

impl EscrowProxyExt for SigningAccount {
    async fn deploy_escrow_proxy(&self, roles: RolesConfig, config: ProxyConfig) -> anyhow::Result<()> {
        self.tx(self.id().clone())
            .transfer(NearToken::from_near(5))
            .deploy(ESCROW_PROXY_WASM.clone())
            .function_call(
                FnCallBuilder::new("new")
                    .json_args(json!({
                        "roles": roles,
                        "config": config,
                    }))
                    .with_gas(Gas::from_tgas(50)),
            )
            .await?;

        Ok(())
    }

    async fn get_escrow_proxy_config(&self) -> anyhow::Result<ProxyConfig> {
        self.call_view_function_json("config", json!({})).await
    }
}

// ============================================================================
// EscrowSwapAccountExt
// ============================================================================

#[allow(async_fn_in_trait)]
pub trait EscrowSwapAccountExt {
    /// Deploy global escrow-swap contract (shared code)
    async fn deploy_escrow_swap_global(&self, name: impl AsRef<str>) -> AccountId;

    /// Deploy an escrow-swap instance with specific params using `state_init`
    async fn deploy_escrow_swap_instance(
        &self,
        global_contract_id: AccountId,
        params: &defuse_escrow_swap::Params,
    ) -> AccountId;
}

impl EscrowSwapAccountExt for SigningAccount {
    async fn deploy_escrow_swap_global(&self, name: impl AsRef<str>) -> AccountId {
        let account = self.subaccount(name);

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(50))
            .deploy_global(
                ESCROW_SWAP_WASM.clone(),
                GlobalContractDeployMode::AccountId,
            )
            .await
            .unwrap();

        account.id().clone()
    }

    async fn deploy_escrow_swap_instance(
        &self,
        global_contract_id: AccountId,
        params: &defuse_escrow_swap::Params,
    ) -> AccountId {
        let raw_state = defuse_escrow_swap::ContractStorage::init_state(params).unwrap();
        let state_init = StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(global_contract_id.clone()),
            data: raw_state.clone(),
        });
        let account_id = state_init.derive_account_id();

        // Note: RPC may error but contract deploys successfully
        let _ = self
            .tx(account_id.clone())
            .state_init(global_contract_id, raw_state)
            .transfer(NearToken::from_yoctonear(1))
            .await;
        account_id
    }
}

// ============================================================================
// TransferAuthAccountExt
// ============================================================================

#[allow(async_fn_in_trait)]
pub trait TransferAuthAccountExt {
    async fn deploy_transfer_auth(&self, name: impl AsRef<str>) -> AccountId;
    async fn deploy_transfer_auth_instance(
        &self,
        global_contract_id: AccountId,
        state: State,
    ) -> AccountId;
    async fn get_transfer_auth_instance_state(
        &self,
        global_contract_id: AccountId,
    ) -> anyhow::Result<ContractStorage>;
}

impl TransferAuthAccountExt for SigningAccount {
    async fn deploy_transfer_auth(&self, name: impl AsRef<str>) -> AccountId {
        let account = self.sub_account(name).unwrap();

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(20))
            .deploy_global(
                TRANSFER_AUTH_WASM.clone(),
                GlobalContractDeployMode::AccountId,
            )
            .await
            .unwrap();

        account.id().clone()
    }

    async fn deploy_transfer_auth_instance(
        &self,
        global_contract_id: AccountId,
        state: State,
    ) -> AccountId {
        let raw_state = ContractStorage::init_state(state.clone()).unwrap();
        let solver1_state_init = StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(global_contract_id.clone()),
            data: raw_state.clone(),
        });

        let account = solver1_state_init.derive_account_id();

        // NOTE: there is rpc error on state_init action but the contract itself is successfully
        // deployed, so lets ignore error for now
        let _ = self
            .tx(account.clone())
            .state_init(global_contract_id, raw_state)
            .transfer(NearToken::from_yoctonear(1))
            .await;
        account
    }

    async fn get_transfer_auth_instance_state(
        &self,
        global_contract_id: AccountId,
    ) -> anyhow::Result<ContractStorage> {
        let account = Account::new(global_contract_id, self.network_config().clone());
        account.call_view_function_json("state", json!({})).await
    }
}

