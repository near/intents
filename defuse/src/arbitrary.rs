use defuse_actions::{AppendAction, FunctionCallAction, TransferAction};
use near_sdk::{AccountId, Promise, ext_contract, near};

#[near(serializers = [json])]
pub enum ArbitraryAction {
    FunctionCall(FunctionCallAction),
    Transfer(TransferAction),
}

impl AppendAction for ArbitraryAction {
    fn append(self, p: Promise) -> Promise {
        match self {
            Self::FunctionCall(a) => a.append(p),
            Self::Transfer(a) => a.append(p),
        }
    }
}

#[ext_contract(ext_arbitrary_manager)]
#[allow(clippy::module_name_repetitions)]
pub trait ArbitraryManager {
    /// Allows the caller to execute an arbitrary function call
    /// or transfer on the contract.
    fn arbitrary_call(&mut self, account_id: AccountId, action: ArbitraryAction);
}
