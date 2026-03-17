use std::collections::BTreeSet;

use defuse_wallet::{Request, signature::Deadline, signature::RequestMessage};
use near_sdk::{
    AccountId,
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ExecuteSignedArgs {
    pub msg: RequestMessage,
    pub proof: String,
}

#[near_kit::contract]
pub trait Wallet {
    #[call]
    fn w_execute_signed(&mut self, args: ExecuteSignedArgs) -> bool;

    #[call]
    fn w_execute_extension(&mut self, request: Request) -> bool;

    fn w_subwallet_id(&self) -> u32;
    fn w_is_signature_allowed(&self) -> bool;
    fn w_public_key(&self) -> String;
    fn w_is_extension_enabled(&self, account_id: AccountId) -> bool;
    fn w_extensions(&self) -> BTreeSet<AccountId>;
    fn w_timeout_secs(&self) -> u64;
    fn w_last_cleaned_at(&self) -> Deadline;
}
