use base64::{Engine as _, engine::general_purpose::STANDARD};
use defuse_nep245::{TokenId, receiver::MultiTokenReceiver};
use near_sdk::{
    AccountId, PromiseOrValue,
    json_types::U128,
    near,
    serde::{Deserialize, Serialize},
    serde_json,
};

/// Minimal stub contract used for integration tests.
#[near(contract_state)]
pub struct Contract {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum StubAction {
    ReturnValue(U128),
}

#[near]
impl MultiTokenReceiver for Contract {
    fn mt_on_transfer(
        &mut self,
        _sender_id: AccountId,
        _previous_owner_ids: Vec<AccountId>,
        _token_ids: Vec<TokenId>,
        _amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        let action: StubAction = serde_json::from_str(&msg).unwrap();
        match action{
            StubAction::ReturnValue(value) => PromiseOrValue::Value(vec![value]),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn mt_on_transfer_returns_requested_value() {
        let mut contract = Contract {};
        let message = serde_json::to_string(&StubAction::ReturnValue(U128(42))).unwrap();

        let PromiseOrValue::Value(result) = contract.mt_on_transfer(
            AccountId::from_str("sender.testnet").unwrap(),
            vec![],
            vec!["token".to_string()],
            vec![U128(1)],
            message,
        ) else {
            panic!("expected value promise");
        };

        assert_eq!(result, vec![U128(42)]);
    }
}
