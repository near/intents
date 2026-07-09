use derive_more::From;
use serde::{Deserialize, Serialize};

use crate::{OverrideSend, Params, Timestamp, decimal::UD128};

#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferMessage {
    pub params: Params,
    pub action: TransferAction,
}

#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "action", content = "data", rename_all = "snake_case")]
pub enum TransferAction {
    Fund,
    Fill(FillAction),
    // TODO: Borrow, Repay
}

/// NOTE: make sure you (or `receiver_id`) has enough `storage_deposit`
/// on `src_token`, otherwise tokens will be lost.
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillAction {
    pub price: UD128,

    pub deadline: Timestamp,

    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub receive_src_to: OverrideSend,
}
