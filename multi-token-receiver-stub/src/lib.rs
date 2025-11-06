use defuse_nep245::{TokenId, receiver::MultiTokenReceiver};
use near_sdk::{
    AccountId, PromiseOrValue, env,
    json_types::U128,
    near,
    serde::{Deserialize, Serialize},
    serde_json,
};

/// Minimal stub contract used for integration tests.
#[derive(Default)]
#[near(contract_state)]
pub struct Contract;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum StubAction {
    ReturnValue(U128),
    Panic,
    MaliciousReturn,
}

impl StubAction {
    fn decode(msg: &str) -> Self {
        serde_json::from_str(msg)
            .unwrap_or_else(|err| panic!("failed to deserialize StubAction: {err}"))
    }

    #[cfg(test)]
    fn encode(&self) -> String {
        serde_json::to_string(self).expect("StubAction::encode serialization should succeed")
    }
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
        match StubAction::decode(&msg) {
            StubAction::ReturnValue(value) => PromiseOrValue::Value(vec![value]),
            StubAction::Panic => env::panic_str("StubAction::Panic"),
            StubAction::MaliciousReturn => {
                PromiseOrValue::Value(vec![U128(0xffffffffffffffffffffffffffffffff); 250000])
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
        let message = StubAction::ReturnValue(U128(42)).encode();

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
