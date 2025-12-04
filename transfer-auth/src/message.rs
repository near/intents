use near_sdk::{
    serde::{Deserialize, Serialize},
    AccountId, CryptoHash,
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct AuthMessage {
    pub solver_id: AccountId,
    pub escrow_params_hash: CryptoHash,
}
