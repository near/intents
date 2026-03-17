use near_sdk::{
    AccountId,
    json_types::U128,
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct GDApproveArgs {
    pub token: String,
    pub owner_id: AccountId,
    pub amount: U128,
    pub msg: Option<String>,
    pub memo: Option<String>,
}

#[near_kit::contract]
pub trait GlobalDeployer {
    #[call]
    fn gd_approve(&mut self, args: GDApproveArgs) -> bool;

    #[call]
    fn gd_deploy(&mut self, code: &[u8]) -> bool;

    #[call]
    fn gd_transfer_ownership(&mut self, receiver_id: AccountId);

    fn gd_owner_id(&self) -> AccountId;
    fn gd_code_hash(&self) -> [u8; 32];
    fn gd_approved_hash(&self) -> [u8; 32];
}
