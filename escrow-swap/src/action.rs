use derive_more::From;
use near_sdk::near;

use crate::{Deadline, OverrideSend, Params, decimal::UD128};

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct TransferMessage {
    pub params: Params,
    pub action: TransferAction,
}

#[near(serializers = [json])]
#[serde(tag = "action", content = "data", rename_all = "snake_case")]
#[derive(Debug, Clone, From)]
pub enum TransferAction {
    Fund,
    Fill(FillAction),
    // TODO: Borrow, Repay
}

/// NOTE: make sure you (or receiver_id) has enough `storage_deposit`
/// on `src_token`, otherwise tokens will be lost.
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FillAction {
    pub price: UD128,

    pub deadline: Deadline,

    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub receive_src_to: OverrideSend,
}
