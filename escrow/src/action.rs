use derive_more::From;
use near_sdk::{AccountId, near};

use crate::{FixedParams, Price};

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct TransferMessage {
    pub fixed_params: FixedParams,
    pub action: Action,
}

#[near(serializers = [json])]
#[serde(tag = "type", content = "data")]
#[derive(Debug, Clone, From)]
pub enum Action {
    Open(OpenAction),
    Fill(FillAction),
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct OpenAction {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_price: Option<Price>,
    // TODO: exact_out support?
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FillAction {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver_id: Option<AccountId>,
}
