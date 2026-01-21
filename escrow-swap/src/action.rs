use derive_more::From;
use near_sdk::near;

use crate::{DEFAULT_DEADLINE_SECS, Deadline, OverrideSend, Params, decimal::UD128};

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

pub struct FundMessageBuilder {
    params: Params,
}

impl FundMessageBuilder {
    #[must_use]
    pub const fn new(params: Params) -> Self {
        Self { params }
    }

    #[must_use]
    pub fn build(self) -> TransferMessage {
        TransferMessage {
            params: self.params,
            action: TransferAction::Fund,
        }
    }
}

/// Builder for creating Fill transfer messages.
pub struct FillMessageBuilder {
    params: Params,
    price: Option<UD128>,
    deadline: Option<Deadline>,
    receive_src_to: Option<OverrideSend>,
}

impl FillMessageBuilder {
    #[must_use]
    pub const fn new(params: Params) -> Self {
        Self {
            params,
            price: None,
            deadline: None,
            receive_src_to: None,
        }
    }

    #[must_use]
    pub const fn with_price(mut self, price: UD128) -> Self {
        self.price = Some(price);
        self
    }

    #[must_use]
    pub const fn with_deadline(mut self, deadline: Deadline) -> Self {
        self.deadline = Some(deadline);
        self
    }

    #[must_use]
    pub fn with_receive_src_to(mut self, receive_src_to: OverrideSend) -> Self {
        self.receive_src_to = Some(receive_src_to);
        self
    }

    #[must_use]
    pub fn build(self) -> TransferMessage {
        TransferMessage {
            params: self.params.clone(),
            action: TransferAction::Fill(FillAction {
                price: self.price.unwrap_or(self.params.price),
                deadline: self.deadline.unwrap_or_else(|| {
                    Deadline::timeout(std::time::Duration::from_secs(DEFAULT_DEADLINE_SECS))
                }),
                receive_src_to: self.receive_src_to.unwrap_or_default(),
            }),
        }
    }
}
