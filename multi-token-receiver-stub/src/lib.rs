use defuse_nep245::{TokenId, receiver::MultiTokenReceiver};
use near_sdk::{
    AccountId, PromiseOrValue, env,
    json_types::U128,
    near,
    serde_json,
};

/// Minimal stub contract used for integration tests.
#[derive(Default)]
#[near(contract_state)]
pub struct Contract;

#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StubAction {
    ReturnValue(U128),
    ReturnValues(Vec<U128>),
    Panic,
    MaliciousReturn,
}

#[near]
impl MultiTokenReceiver for Contract {
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        near_sdk::env::log_str(&format!(
            "STUB::mt_on_transfer: sender_id={sender_id}, previous_owner_ids={previous_owner_ids:?}, token_ids={token_ids:?}, amounts={amounts:?}, msg={msg}"
        ));
        let action: StubAction = serde_json::from_str(&msg)
            .unwrap_or_else(|err| panic!("failed to deserialize StubAction: {err}"));
        match action {
            StubAction::ReturnValue(value) => {
                // Return the same refund value for each token
                PromiseOrValue::Value(vec![value; amounts.len()])
            }
            StubAction::ReturnValues(values) => {
                // Return specific refund values for each token
                PromiseOrValue::Value(values)
            }
            StubAction::Panic => env::panic_str("StubAction::Panic"),
            StubAction::MaliciousReturn => {
                PromiseOrValue::Value(vec![U128(0xffffffffffffffffffffffffffffffff); 2500000])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn mt_on_transfer_returns_requested_value() {
        let mut contract = Contract;
        let message = serde_json::to_string(&StubAction::ReturnValue(U128(42)))
            .expect("serialization should succeed");

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
