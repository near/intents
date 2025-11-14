#[cfg(feature = "nep141")]
mod nep141;
#[cfg(feature = "nep245")]
mod nep245;

use defuse_token_id::{TokenId, TokenIdType};
use derive_more::From;
use near_sdk::{Gas, near};

use crate::state::{OverrideSend, Params};

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
    Open,
    Fill(FillAction),
    // Borrow(BorrowAction),
    // Repay(RepayAction),
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FillAction {
    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub receive_src_to: OverrideSend,
    // TODO: min_src_out?
}

// #[near(serializers = [json])]
// #[derive(Debug, Clone)]
// pub struct BorrowAction {

// }

// #[near(serializers = [json])]
// #[derive(Debug, Clone)]
// pub struct RepayAction {}

pub trait TokenIdExt: Sized {
    fn token_type(&self) -> TokenIdType;

    fn transfer_gas_min_default(&self, is_call: bool) -> (Gas, Gas);

    fn transfer_gas(&self, min_gas: Option<Gas>, is_call: bool) -> Gas {
        let (min, default) = self.transfer_gas_min_default(is_call);
        min_gas.unwrap_or(default).max(min)
    }
}

impl TokenIdExt for TokenId {
    #[inline]
    fn token_type(&self) -> TokenIdType {
        self.into()
    }

    fn transfer_gas_min_default(&self, is_call: bool) -> (Gas, Gas) {
        match self {
            #[cfg(feature = "nep141")]
            Self::Nep141(token) => token.transfer_gas_min_default(is_call),
            #[cfg(feature = "nep245")]
            Self::Nep245(token) => token.transfer_gas_min_default(is_call),
        }
    }
}
