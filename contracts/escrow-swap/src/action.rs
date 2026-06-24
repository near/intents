use defuse_serde_utils::jiff::Rfc3339;
use derive_more::From;
use near_sdk::near;

use crate::{OverrideSend, Params, Timestamp, decimal::UD128};

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

/// NOTE: make sure you (or `receiver_id`) has enough `storage_deposit`
/// on `src_token`, otherwise tokens will be lost.
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FillAction {
    pub price: UD128,

    #[serde_as(as = "Rfc3339")]
    pub deadline: Timestamp,

    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub receive_src_to: OverrideSend,
}
