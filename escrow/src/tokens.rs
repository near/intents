use derive_more::From;
use near_sdk::near;

use crate::{
    Price,
    state::{FixedParams, OverrideSend},
};

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct TransferMessage {
    pub fixed_params: FixedParams,
    pub action: TransferAction,
}

#[near(serializers = [json])]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
#[derive(Debug, Clone, From)]
pub enum TransferAction {
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
    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub receive_src_to: OverrideSend,
    // TODO: min_src_out?
}
