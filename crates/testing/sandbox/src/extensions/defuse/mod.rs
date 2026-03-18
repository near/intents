use anyhow::Result;
pub use defuse as contract;

use defuse::{
    core::{Nonce, Salt, fees::Pips, payload::multi::MultiPayload, tokens::imt::ImtTokens},
    intents::SimulationOutput,
};
use defuse_nep245::TokenId;
use defuse_serde_utils::base64::AsBase64;
use near_kit::{AccountId, FunctionCallAction, NearToken, PublicKey};
use near_sdk::{
    json_types::U128,
    serde::{Deserialize, Serialize},
};
use serde_json::json;

use crate::{DEFAULT_GAS, Sandbox};

#[near_kit::contract]
pub trait Defuse {
    // Fees management
    #[call]
    fn set_fee(&mut self, fee: Pips);
    #[call]
    fn set_fee_collector(&mut self, fee_collector: AccountId);
    fn fee(&self) -> Pips;
    fn fee_collector(&self) -> AccountId;

    // Garbage collector
    #[call]
    fn cleanup_nonces(&mut self, nonces: Vec<(AccountId, Vec<AsBase64<Nonce>>)>);

    // Salts management
    #[call]
    fn update_current_salt(&mut self) -> Salt;
    #[call]
    fn invalidate_salts(&mut self, salts: Vec<Salt>) -> Salt;
    fn is_valid_salt(&self, salt: Salt) -> bool;
    fn current_salt(&self) -> Salt;

    // Accounts management
    fn has_public_key(&self, args: DefusePublicKeyArgs) -> bool;
    fn public_keys_of(&self, account_id: AccountId) -> Vec<String>;
    #[call]
    fn add_public_key(&mut self, public_key: String);
    #[call]
    fn remove_public_key(&mut self, public_key: String);
    fn is_nonce_used(&self, args: DefuseNonceArgs) -> bool;
    fn is_auth_by_predecessor_id_enabled(&self, account_id: AccountId) -> bool;
    #[call]
    fn disable_auth_by_predecessor_id(&mut self);

    // Force management
    fn is_account_locked(&self, account_id: AccountId) -> bool;
    #[call]
    fn force_lock_account(&mut self, account_id: AccountId) -> bool;
    #[call]
    fn force_unlock_account(&mut self, account_id: AccountId) -> bool;
    #[call]
    fn force_disable_auth_by_predecessor_ids(&mut self, account_ids: Vec<AccountId>);
    #[call]
    fn force_enable_auth_by_predecessor_ids(&mut self, account_ids: Vec<AccountId>);

    // Intents
    #[call]
    fn execute_intents(&mut self, signed: Vec<MultiPayload>);
    fn simulate_intents(&self, signed: Vec<MultiPayload>) -> SimulationOutput;

    // Relayer keys management
    #[call]
    fn add_relayer_key(&mut self, public_key: PublicKey);
    #[call]
    fn delete_relayer_key(&mut self, public_key: PublicKey);

    // Imt
    #[call]
    fn imt_burn(&mut self, args: DefuseImtArgs);

    // Ft
    #[call]
    fn ft_withdraw(&mut self, args: FtWithdrawArgs);
    #[call]
    fn ft_force_withdraw(&mut self, args: FtForceWithdrawArgs);

    // Ntf
    #[call]
    fn nft_withdraw(&mut self, args: NftWithdrawArgs);
    #[call]
    fn nft_force_withdraw(&mut self, args: NftForceWithdrawArgs);

    // Mt

    #[call]
    fn mt_withdraw(&mut self, args: MtWithdrawArgs);
    #[call]
    fn mt_force_withdraw(&mut self, args: MtForceWithdrawArgs);

    // nep245
    #[call]
    fn mt_transfer(&mut self, args: MtTransferArgs);
    #[call]
    fn mt_batch_transfer(&mut self, args: MtBatchTransferArgs);
    #[call]
    fn mt_transfer_call(&mut self, args: MtTransferCallArgs);
    #[call]
    fn mt_batch_transfer_call(&mut self, args: MtBatchTransferCallArgs);

    fn mt_balance_of(&self, args: MtBalanceArgs) -> U128;
    fn mt_batch_balance_of(&self, args: MtBatchBalanceArgs) -> Vec<U128>;

    // Mt
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MtTransferArgs {
    pub receiver_id: AccountId,
    pub token_id: defuse_nep245::TokenId,
    pub amount: U128,
    pub approval: Option<(AccountId, u64)>,
    pub memo: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MtBatchTransferArgs {
    pub receiver_id: AccountId,
    pub token_ids: Vec<defuse_nep245::TokenId>,
    pub amounts: Vec<U128>,
    pub approvals: Option<Vec<Option<(AccountId, u64)>>>,
    pub memo: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MtTransferCallArgs {
    pub receiver_id: AccountId,
    pub token_id: defuse_nep245::TokenId,
    pub amount: U128,
    pub approval: Option<(AccountId, u64)>,
    pub memo: Option<String>,
    pub msg: String,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MtBatchTransferCallArgs {
    pub receiver_id: AccountId,
    pub token_ids: Vec<defuse_nep245::TokenId>,
    pub amounts: Vec<U128>,
    pub approvals: Option<Vec<Option<(AccountId, u64)>>>,
    pub memo: Option<String>,
    pub msg: String,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MtBalanceArgs {
    pub account_id: AccountId,
    pub token_id: defuse_nep245::TokenId,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MtBatchBalanceArgs {
    pub account_id: AccountId,
    pub token_ids: Vec<defuse_nep245::TokenId>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct DefusePublicKeyArgs {
    pub account_id: AccountId,
    pub public_key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct DefuseNonceArgs {
    pub account_id: AccountId,
    pub nonce: AsBase64<Nonce>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct DefuseImtArgs {
    pub minter_id: AccountId,
    pub tokens: ImtTokens,
    pub memo: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct FtWithdrawArgs {
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub amount: U128,
    pub memo: Option<String>,
    pub msg: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct FtForceWithdrawArgs {
    pub owner_id: AccountId,
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub amount: U128,
    pub memo: Option<String>,
    pub msg: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct NftWithdrawArgs {
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub token_id: TokenId,
    pub memo: Option<String>,
    pub msg: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct NftForceWithdrawArgs {
    pub owner_id: AccountId,
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub token_id: TokenId,
    pub memo: Option<String>,
    pub msg: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MtWithdrawArgs {
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub token_ids: Vec<TokenId>,
    pub amounts: Vec<U128>,
    pub memo: Option<String>,
    pub msg: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MtForceWithdrawArgs {
    pub owner_id: AccountId,
    pub receiver_id: AccountId,
    pub token_id: TokenId,
    pub amount: U128,
    pub approval: Option<(AccountId, u64)>,
    pub memo: Option<String>,
}

impl Sandbox {
    pub async fn deploy_defuse(
        &self,
        name: impl AsRef<str>,
        config: defuse::contract::config::DefuseConfig,
        wasm: impl Into<Vec<u8>>,
    ) -> Result<DefuseClient> {
        let contract_id = self
            .deploy_sub_contract(
                name,
                NearToken::from_near(100),
                wasm,
                Some(FunctionCallAction {
                    method_name: "new".to_string(),
                    args: json!({
                        "config": config,
                    })
                    .to_string()
                    .into_bytes(),
                    gas: DEFAULT_GAS,
                    deposit: NearToken::from_near(0),
                }),
            )
            .await?;

        Ok(self.contract::<dyn Defuse>(contract_id))
    }
}
