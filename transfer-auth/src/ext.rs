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
}
