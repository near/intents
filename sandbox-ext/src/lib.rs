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
pub static WNEAR_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("../releases/wnear"));
pub static VERIFIER_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("defuse"));
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

// ============================================================================
// DefuseAccountExt (Defuse-specific methods only)
// ============================================================================
// NOTE: For FT/MT operations, use sandbox extensions directly:
// - WNearExt::near_deposit()
// - StorageManagementExt::storage_deposit()
// - FtExt::ft_transfer_call()
// - MtViewExt::mt_balance_of() (on Account)
// - MtExt::mt_transfer_call()

#[allow(async_fn_in_trait)]
pub trait DefuseAccountExt {
    // Contract deployment
    async fn deploy_wnear(&self, name: impl AsRef<str>) -> Account;
    async fn deploy_verifier(&self, name: impl AsRef<str>, wnear_id: AccountId) -> Self;

    // Transfer-auth specific
    async fn call_on_auth(
        &self,
        transfer_auth_instance: &AccountId,
        signer_id: &AccountId,
        msg: &str,
    ) -> anyhow::Result<()>;

    // Defuse public key management
    async fn defuse_add_public_key(
        &self,
        defuse: &Account,
        public_key: defuse_crypto::PublicKey,
    ) -> anyhow::Result<()>;

    async fn defuse_has_public_key(
        defuse: &Account,
        account_id: &AccountId,
        public_key: &defuse_crypto::PublicKey,
    ) -> anyhow::Result<bool>;

    async fn defuse_current_salt(defuse: &Account) -> anyhow::Result<[u8; 32]>;

    // Intent signing and execution
    fn sign_auth_call_intent(
        signer_id: &AccountId,
        secret_key: &[u8; 32],
        defuse_contract_id: &AccountId,
        transfer_auth_global: &AccountId,
        state: &State,
        nonce: [u8; 32],
    ) -> SignedNep413Payload;

    async fn execute_auth_call_intent(
        &self,
        defuse: &SigningAccount,
        transfer_auth_global: &AccountId,
        state: &State,
        secret_key: &[u8; 32],
        nonce: [u8; 32],
    ) -> AccountId;

    async fn execute_transfer_intent(
        &self,
        defuse: &SigningAccount,
        transfer: defuse_core::intents::tokens::Transfer,
        secret_key: &[u8; 32],
        nonce: [u8; 32],
    ) -> anyhow::Result<()>;

    async fn execute_signed_intents(
        &self,
        defuse: &Account,
        payloads: &[defuse_core::payload::multi::MultiPayload],
    ) -> anyhow::Result<()>;
}

impl DefuseAccountExt for SigningAccount {
    async fn deploy_wnear(&self, name: impl AsRef<str>) -> Account {
        let account = self.subaccount(name);

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(20))
            .deploy(WNEAR_WASM.clone())
            .function_call(FnCallBuilder::new("new").with_gas(Gas::from_tgas(50)))
            .await
            .unwrap();

        account
    }

    async fn deploy_verifier(&self, name: impl AsRef<str>, wnear_id: AccountId) -> Self {
        let defuse = self
            .create_subaccount(name, NearToken::from_near(20))
            .await
            .unwrap();

        defuse
            .tx(defuse.id().clone())
            .deploy(VERIFIER_WASM.clone())
            .function_call(
                FnCallBuilder::new("new")
                    .json_args(json!({
                        "config": json!({
                            "wnear_id": wnear_id,
                            "fees": {
                                "fee": defuse_fees::Pips::from_percent(1).unwrap(),
                                "fee_collector": self.id().clone(),
                            },
                        }),
                    }))
                    .with_gas(Gas::from_tgas(50)),
            )
            .await
            .unwrap();

        defuse
    }

    async fn call_on_auth(
        &self,
        transfer_auth_instance: &AccountId,
        signer_id: &AccountId,
        msg: &str,
    ) -> anyhow::Result<()> {
        self.tx(transfer_auth_instance.clone())
            .function_call(
                FnCallBuilder::new("on_auth")
                    .json_args(json!({
                        "signer_id": signer_id,
                        "msg": msg,
                    }))
                    .with_gas(Gas::from_tgas(50))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await
            .map(|_| ())
    }

    async fn defuse_add_public_key(
        &self,
        defuse: &Account,
        public_key: defuse_crypto::PublicKey,
    ) -> anyhow::Result<()> {
        self.tx(defuse.id().clone())
            .function_call(
                FnCallBuilder::new("add_public_key")
                    .json_args(json!({
                        "public_key": public_key,
                    }))
                    .with_gas(Gas::from_tgas(50))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await
            .map(|_| ())
    }

    async fn defuse_has_public_key(
        defuse: &Account,
        account_id: &AccountId,
        public_key: &defuse_crypto::PublicKey,
    ) -> anyhow::Result<bool> {
        defuse
            .call_view_function_json(
                "has_public_key",
                json!({
                    "account_id": account_id,
                    "public_key": public_key,
                }),
            )
            .await
    }

    async fn defuse_current_salt(defuse: &Account) -> anyhow::Result<[u8; 32]> {
        defuse
            .call_view_function_json::<[u8; 32]>("current_salt", json!({}))
            .await
    }

    fn sign_auth_call_intent(
        signer_id: &AccountId,
        secret_key: &[u8; 32],
        defuse_contract_id: &AccountId,
        transfer_auth_global: &AccountId,
        state: &State,
        nonce: [u8; 32],
    ) -> SignedNep413Payload {
        use defuse_core::intents::{DefuseIntents, Intent, auth::AuthCall};
        use defuse_core::payload::nep413::Nep413DefuseMessage;

        let raw_state = ContractStorage::init_state(state.clone()).unwrap();
        let state_init = StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(transfer_auth_global.clone()),
            data: raw_state,
        });
        let instance_id = state_init.derive_account_id();

        let deadline = Deadline::timeout(std::time::Duration::from_secs(120));

        let auth_call = AuthCall {
            contract_id: instance_id,
            state_init: Some(state_init),
            msg: String::new(),
            attached_deposit: NearToken::from_yoctonear(0),
            min_gas: None,
        };

        let nep413_message = Nep413DefuseMessage {
            signer_id: signer_id.clone(),
            deadline,
            message: DefuseIntents {
                intents: vec![Intent::AuthCall(auth_call)],
            },
        };

        let nep413_payload = Nep413Payload::new(serde_json::to_string(&nep413_message).unwrap())
            .with_recipient(defuse_contract_id)
            .with_nonce(nonce);

        let hash = nep413_payload.hash();
        let (public_key, signature) = sign_ed25519(secret_key, &hash);

        SignedNep413Payload {
            payload: nep413_payload,
            public_key,
            signature,
        }
    }

    async fn execute_auth_call_intent(
        &self,
        defuse: &SigningAccount,
        transfer_auth_global: &AccountId,
        state: &State,
        secret_key: &[u8; 32],
        nonce: [u8; 32],
    ) -> AccountId {
        use defuse_core::payload::multi::MultiPayload;

        let signed_payload = Self::sign_auth_call_intent(
            self.id(),
            secret_key,
            defuse.id(),
            transfer_auth_global,
            state,
            nonce,
        );

        let raw_state = ContractStorage::init_state(state.clone()).unwrap();
        let state_init = StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(transfer_auth_global.clone()),
            data: raw_state,
        });
        let instance_id = state_init.derive_account_id();

        let multi_payload = MultiPayload::Nep413(signed_payload);

        // Note: RPC may return parsing error but the tx succeeds
        let _ = self
            .tx(defuse.id().clone())
            .function_call(
                FnCallBuilder::new("execute_intents")
                    .json_args(json!({ "signed": [multi_payload] }))
                    .with_gas(Gas::from_tgas(300)),
            )
            .await;

        instance_id
    }

    async fn execute_transfer_intent(
        &self,
        defuse: &SigningAccount,
        transfer: defuse_core::intents::tokens::Transfer,
        secret_key: &[u8; 32],
        nonce: [u8; 32],
    ) -> anyhow::Result<()> {
        use defuse_core::intents::{DefuseIntents, Intent};
        use defuse_core::payload::multi::MultiPayload;
        use defuse_core::payload::nep413::Nep413DefuseMessage;

        let deadline = Deadline::timeout(std::time::Duration::from_secs(120));

        let nep413_message = Nep413DefuseMessage {
            signer_id: self.id().clone(),
            deadline,
            message: DefuseIntents {
                intents: vec![Intent::Transfer(transfer)],
            },
        };

        let nep413_payload = Nep413Payload::new(serde_json::to_string(&nep413_message).unwrap())
            .with_recipient(defuse.id())
            .with_nonce(nonce);

        let hash = nep413_payload.hash();
        let (public_key, signature) = sign_ed25519(secret_key, &hash);

        let signed_payload = SignedNep413Payload {
            payload: nep413_payload,
            public_key,
            signature,
        };

        let multi_payload = MultiPayload::Nep413(signed_payload);

        // reports error but goes through
        let _ = self
            .tx(defuse.id().clone())
            .function_call(
                FnCallBuilder::new("execute_intents")
                    .json_args(json!({ "signed": [multi_payload] }))
                    .with_gas(Gas::from_tgas(300)),
            )
            .await;

        Ok(())
    }

    async fn execute_signed_intents(
        &self,
        defuse: &Account,
        payloads: &[defuse_core::payload::multi::MultiPayload],
    ) -> anyhow::Result<()> {
        // Note: RPC may return parsing error but the tx succeeds
        let _ = self
            .tx(defuse.id().clone())
            .function_call(
                FnCallBuilder::new("execute_intents")
                    .json_args(json!({ "signed": payloads }))
                    .with_gas(Gas::from_tgas(300)),
            )
            .await;

        Ok(())
    }
}
