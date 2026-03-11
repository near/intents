use defuse_actions::{Action, FunctionCallAction, TransferAction};
use near_sdk::{AccountId, NearToken, Promise, ext_contract, near};

#[near(serializers = [json])]
#[derive(Clone, Debug)]
pub enum ArbitraryAction {
    FunctionCall(FunctionCallAction),
    Transfer(TransferAction),
}

impl Action for ArbitraryAction {
    fn append(self, p: Promise) -> Promise {
        match self {
            Self::FunctionCall(a) => a.append(p),
            Self::Transfer(a) => a.append(p),
        }
    }

    fn get_deposit(&self) -> NearToken {
        match self {
            Self::FunctionCall(a) => a.get_deposit(),
            Self::Transfer(_) => NearToken::ZERO, // transfer action doesn't require deposit
        }
    }
}

#[ext_contract(ext_arbitrary_manager)]
pub trait ArbitraryManager {
    /// Allows the caller to execute an arbitrary function call
    /// or transfer on the contract.
    fn arbitrary_call(&mut self, account_id: AccountId, action: ArbitraryAction) -> Promise;
}
