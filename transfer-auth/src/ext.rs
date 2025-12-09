use std::{fs, path::Path, sync::LazyLock};

use crate::storage::{ContractStorage, State};
use serde_json::json;
use defuse_sandbox::{
    api::types::transaction::actions::GlobalContractDeployMode, Account, SigningAccount, TxError,
};
use defuse_token_id::TokenId;
use near_sdk::{
    AccountId, Gas, GlobalContractId, NearToken,
    json_types::U128,
    state_init::{StateInit, StateInitV1},
};
use defuse_deadline::Deadline;
use defuse_nep413::{Nep413Payload, SignedNep413Payload};
use defuse_crypto::Payload;

/// Sign a message with Ed25519 using a raw 32-byte secret key.
/// Returns (public_key, signature) as raw byte arrays.
pub fn sign_ed25519(secret_key: &[u8; 32], message: &[u8]) -> ([u8; 32], [u8; 64]) {
    use ed25519_dalek::{Signer, SigningKey};
    let signing_key = SigningKey::from_bytes(secret_key);
    let public_key = signing_key.verifying_key().to_bytes();
    let signature = signing_key.sign(message).to_bytes();
    (public_key, signature)
}

/// Hardcoded test private key (32 bytes) - FOR TESTING ONLY
pub const TEST_SECRET_KEY: [u8; 32] = [
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
    0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
];

/// Get the public key for the hardcoded test secret key
pub fn test_public_key() -> defuse_crypto::PublicKey {
    let (pk, _) = sign_ed25519(&TEST_SECRET_KEY, &[]);
    defuse_crypto::PublicKey::Ed25519(pk)
}

#[track_caller]
fn read_wasm(name: impl AsRef<Path>) -> Vec<u8> {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../res/")
        .join(name)
        .with_extension("wasm");
    fs::read(filename.clone()).expect(&format!("file {filename:?} should exists"))
}

pub static WNEAR_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("../tests/contracts/target/wnear"));
pub static VERIFIER_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("defuse"));
pub static TRANSFER_AUTH_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("defuse_transfer_auth"));

/// Derive the transfer-auth instance account ID from its state
pub fn derive_transfer_auth_account_id(global_contract_id: &AccountId, state: &State) -> AccountId {
    let raw_state = ContractStorage::init_state(state.clone()).unwrap();
    let state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(global_contract_id.clone()),
        data: raw_state,
    });
    state_init.derive_account_id()
}

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
        let account = self.subaccount(name);

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(100))
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
            code: near_sdk::GlobalContractId::AccountId(global_contract_id.clone()),
            data: raw_state.clone(),
        });

        let account = solver1_state_init.derive_account_id();

        //NOTE: there is rpc error on state_init action but the contract itself is successfully
        //deployed, so lets ignore error for now
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
        Ok(self
            .tx(global_contract_id)
            .function_call_json::<ContractStorage>(
                "state",
                "{}",
                Gas::from_tgas(300),
                NearToken::from_near(0),
            )
            .await?)
    }
}


// TODO: move to defuse
pub trait DefuseAccountExt {
    async fn deploy_wnear(&self, name: impl AsRef<str>) -> Account;
    async fn deploy_verifier(&self, name: impl AsRef<str>, wnear_id: AccountId) -> SigningAccount;

    // WNEAR operations
    async fn near_deposit(&self, wnear: &Account, amount: NearToken) -> Result<(), TxError>;
    async fn ft_storage_deposit(
        &self,
        token: &Account,
        account_id: Option<&AccountId>,
    ) -> Result<(), TxError>;
    async fn ft_transfer_call(
        &self,
        token: &Account,
        receiver_id: &AccountId,
        amount: u128,
        msg: &str,
    ) -> Result<u128, TxError>;

    // Query MT balance
    async fn mt_balance_of(
        defuse: &Account,
        account_id: &AccountId,
        token_id: &TokenId,
    ) -> anyhow::Result<u128>;

    // MT transfer call
    async fn mt_transfer_call(
        &self,
        defuse: &Account,
        receiver_id: &AccountId,
        token_id: &TokenId,
        amount: u128,
        msg: &str,
    ) -> Result<Vec<u128>, TxError>;

    /// Call on_auth on a transfer-auth contract instance.
    /// NOTE: In production, this should be done via AuthCall intent through defuse.
    /// This direct call is for testing purposes only.
    async fn call_on_auth(
        &self,
        transfer_auth_instance: &AccountId,
        signer_id: &AccountId,
        msg: &str,
    ) -> Result<(), TxError>;

    /// Register a public key for the caller's account in the defuse contract.
    /// This allows intents signed with the corresponding private key to be verified.
    async fn defuse_add_public_key(
        &self,
        defuse: &Account,
        public_key: defuse_crypto::PublicKey,
    ) -> Result<(), TxError>;

    /// Get current salt from defuse for nonce generation
    async fn defuse_current_salt(defuse: &Account) -> anyhow::Result<[u8; 32]>;

    /// Sign an AuthCall intent with state_init using NEP-413 standard.
    /// Returns a typed SignedNep413Payload ready to be passed to execute_intents.
    fn sign_auth_call_intent(
        signer_id: &AccountId,
        secret_key: &[u8; 32],
        defuse_contract_id: &AccountId,
        transfer_auth_global: &AccountId,
        state: &State,
        nonce: [u8; 32],
    ) -> SignedNep413Payload;

    /// Execute an AuthCall intent with state_init to deploy transfer-auth and authorize.
    ///
    /// This method:
    /// 1. Creates a new keypair for signing
    /// 2. Registers the public key in defuse for this account
    /// 3. Creates an AuthCall intent with state_init containing the transfer-auth state
    /// 4. Signs the intent with the new keypair
    /// 5. Executes the intent via defuse's execute_intents
    async fn execute_auth_call_intent(
        &self,
        defuse: &SigningAccount,
        transfer_auth_global: &AccountId,
        state: &State,
    ) -> AccountId;
}

impl DefuseAccountExt for SigningAccount {
    async fn deploy_wnear(&self, name: impl AsRef<str>) -> Account {
        let account = self.subaccount(name);

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(20))
            .deploy(WNEAR_WASM.clone())
            .function_call_json::<()>("new", (), Gas::from_tgas(50), NearToken::from_yoctonear(0))
            .no_result()
            .await
            .unwrap();

        account
    }

    async fn deploy_verifier(&self, name: impl AsRef<str>, wnear_id: AccountId) -> SigningAccount {
        let defuse = self.create_subaccount(name, NearToken::from_near(20)).await.unwrap();

        defuse.tx(defuse.id().clone())
            .deploy(VERIFIER_WASM.clone())
            .function_call_json::<()>(
                "new",
                json!({
                    "config": json!({
                        "wnear_id": wnear_id,
                        "fees": {
                            "fee": defuse_fees::Pips::from_percent(1).unwrap(),
                            "fee_collector": self.id().clone(),
                        },
                    }),
                }),
                Gas::from_tgas(50),
                NearToken::from_yoctonear(0),
            )
            .no_result()
            .await
            .unwrap();

        defuse
    }

    async fn near_deposit(&self, wnear: &Account, amount: NearToken) -> Result<(), TxError> {
        self.tx(wnear.id().clone())
            .function_call_json::<()>("near_deposit", json!({}), Gas::from_tgas(50), amount)
            .no_result()
            .await
    }

    async fn ft_storage_deposit(
        &self,
        token: &Account,
        account_id: Option<&AccountId>,
    ) -> Result<(), TxError> {
        self.tx(token.id().clone())
            .function_call_json::<serde_json::Value>(
                "storage_deposit",
                json!({ "account_id": account_id }),
                Gas::from_tgas(50),
                NearToken::from_millinear(50), // 0.05 NEAR for storage
            )
            .await
            .map(|_| ())
    }

    async fn ft_transfer_call(
        &self,
        token: &Account,
        receiver_id: &AccountId,
        amount: u128,
        msg: &str,
    ) -> Result<u128, TxError> {
        self.tx(token.id().clone())
            .function_call_json::<U128>(
                "ft_transfer_call",
                json!({
                    "receiver_id": receiver_id,
                    "amount": U128(amount),
                    "msg": msg,
                }),
                Gas::from_tgas(100),
                NearToken::from_yoctonear(1),
            )
            .await
            .map(|u| u.0)
    }

    async fn mt_balance_of(
        defuse: &Account,
        account_id: &AccountId,
        token_id: &TokenId,
    ) -> anyhow::Result<u128> {
        defuse
            .call_function_json::<U128>(
                "mt_balance_of",
                json!({
                    "account_id": account_id,
                    "token_id": token_id.to_string(),
                }),
            )
            .await
            .map(|u| u.0)
    }

    async fn mt_transfer_call(
        &self,
        defuse: &Account,
        receiver_id: &AccountId,
        token_id: &TokenId,
        amount: u128,
        msg: &str,
    ) -> Result<Vec<u128>, TxError> {
        self.tx(defuse.id().clone())
            .function_call_json::<Vec<U128>>(
                "mt_transfer_call",
                json!({
                    "receiver_id": receiver_id,
                    "token_id": token_id.to_string(),
                    "amount": U128(amount),
                    "msg": msg,
                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(1),
            )
            .await
            .map(|v| v.into_iter().map(|u| u.0).collect())
    }

    async fn call_on_auth(
        &self,
        transfer_auth_instance: &AccountId,
        signer_id: &AccountId,
        msg: &str,
    ) -> Result<(), TxError> {
        self.tx(transfer_auth_instance.clone())
            .function_call_json::<()>(
                "on_auth",
                json!({
                    "signer_id": signer_id,
                    "msg": msg,
                }),
                Gas::from_tgas(50),
                NearToken::from_yoctonear(1),
            )
            .no_result()
            .await
    }

    async fn defuse_add_public_key(
        &self,
        defuse: &Account,
        public_key: defuse_crypto::PublicKey,
    ) -> Result<(), TxError> {
        self.tx(defuse.id().clone())
            .function_call_json::<()>(
                "add_public_key",
                json!({
                    "public_key": public_key,
                }),
                Gas::from_tgas(50),
                NearToken::from_yoctonear(1),
            )
            .no_result()
            .await
    }

    async fn defuse_current_salt(defuse: &Account) -> anyhow::Result<[u8; 32]> {
        defuse
            .call_function_json::<[u8; 32]>("current_salt", json!({}))
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

        // Create AuthCall intent using the proper typed struct
        let auth_call = AuthCall {
            contract_id: instance_id,
            state_init: Some(state_init),
            msg: String::new(),
            attached_deposit: NearToken::from_yoctonear(0),
            min_gas: None,
        };

        // Create the NEP-413 message structure using proper types
        let nep413_message = Nep413DefuseMessage {
            signer_id: signer_id.clone(),
            deadline,
            message: DefuseIntents {
                intents: vec![Intent::AuthCall(auth_call)],
            },
        };

        // Create NEP-413 payload
        let nep413_payload = Nep413Payload::new(serde_json::to_string(&nep413_message).unwrap())
            .with_recipient(defuse_contract_id)
            .with_nonce(nonce);

        // Hash the payload for signing
        let hash = nep413_payload.hash();

        // Sign using ed25519_dalek
        let (public_key, signature) = sign_ed25519(secret_key, &hash);

        // Return typed SignedNep413Payload
        SignedNep413Payload {
            payload: nep413_payload,
            public_key,
            signature,
        }
    }

    /// Execute an AuthCall intent with state_init to deploy transfer-auth and authorize.
    ///
    /// This method:
    /// 1. Uses a hardcoded test keypair for signing
    /// 2. Registers the public key in defuse for this account
    /// 3. Creates an AuthCall intent with state_init containing the transfer-auth state
    /// 4. Signs the intent with the keypair
    /// 5. Executes the intent via defuse's execute_intents
    async fn execute_auth_call_intent(
        &self,
        defuse: &SigningAccount,
        transfer_auth_global: &AccountId,
        state: &State,
    ) -> AccountId {
        use defuse_core::payload::multi::MultiPayload;

        // 1. Get public key from hardcoded test secret
        let public_key = test_public_key();

        // 2. Register public key in defuse (ignore error if already registered)
        let _ = self.defuse_add_public_key(&**defuse, public_key).await;

        // 3. Get current salt for nonce
        let salt = Self::defuse_current_salt(&**defuse).await.unwrap_or([0u8; 32]);

        // 4. Create unique nonce using timestamp + salt
        let nonce: [u8; 32] = {
            use std::time::{SystemTime, UNIX_EPOCH};
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let mut bytes = [0u8; 32];
            bytes[..16].copy_from_slice(&timestamp.to_le_bytes());
            bytes[16..].copy_from_slice(&salt[..16]);
            bytes
        };

        // 5. Sign the intent
        let signed_payload = Self::sign_auth_call_intent(
            self.id(),
            &TEST_SECRET_KEY,
            defuse.id(),
            transfer_auth_global,
            state,
            nonce,
        );

        // 6. Derive instance ID for return value
        let raw_state = ContractStorage::init_state(state.clone()).unwrap();
        let state_init = StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(transfer_auth_global.clone()),
            data: raw_state,
        });
        let instance_id = state_init.derive_account_id();

        // 7. Wrap in MultiPayload for execute_intents
        let multi_payload = MultiPayload::Nep413(signed_payload);

        // 8. Execute the intent via defuse
        // Note: RPC may return parsing error but the tx succeeds
        let _ = self.tx(defuse.id().clone())
            .function_call_json::<serde_json::Value>(
                "execute_intents",
                json!({ "signed": [multi_payload] }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(0),
            )
            .await;

        instance_id
    }
}
