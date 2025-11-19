use defuse_nep245::{TokenId, receiver::MultiTokenReceiver};
use near_sdk::{AccountId, PromiseOrValue, env, json_types::U128, near, serde_json};

/// Minimal stub contract used for integration tests.
#[derive(Default)]
#[near(contract_state)]
pub struct Contract;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[near(serializers = [json])]
pub enum MTReceiverMode {
    #[default]
    AcceptAll,
    ReturnValue(U128),
    ReturnValues(Vec<U128>),
    Panic,
    LargeReturn,
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
        let mode = serde_json::from_str(&msg).unwrap_or_default();

        match mode {
            MTReceiverMode::ReturnValue(value) => PromiseOrValue::Value(vec![value; amounts.len()]),
            MTReceiverMode::ReturnValues(values) => PromiseOrValue::Value(values),
            MTReceiverMode::AcceptAll => PromiseOrValue::Value(vec![U128(0); amounts.len()]),
            MTReceiverMode::Panic => env::panic_str("MTReceiverMode::Panic"),
            // 16 * 250_000 = 4 MB, which is the limit for a contract return value
            MTReceiverMode::LargeReturn => PromiseOrValue::Value(vec![U128(u128::MAX); 250_000]),
        }
    }
}

// Add backward compatibility variants
impl MTReceiverMode {
    pub const MALICIOUS_RETURN: Self = Self::LargeReturn;

    // Deprecated alias for backwards compatibility
    #[allow(non_upper_case_globals)]
    pub const MaliciousReturn: Self = Self::MALICIOUS_RETURN;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn mt_on_transfer_returns_requested_value() {
        let mut contract = Contract;
        let message = serde_json::to_string(&MTReceiverMode::ReturnValue(U128(42))).unwrap();

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

    #[test]
    fn mt_on_transfer_accept_all_funds() {
        let mut contract = Contract;
        let message = serde_json::to_string(&MTReceiverMode::AcceptAll).unwrap();

        let PromiseOrValue::Value(result) = contract.mt_on_transfer(
            AccountId::from_str("sender.testnet").unwrap(),
            vec![],
            vec!["token".to_string()],
            vec![U128(1)],
            message,
        ) else {
            panic!("expected value promise");
        };

        assert_eq!(result, vec![U128(0)]);
    }

    #[test]
    fn mt_on_transfer_return_values() {
        let mut contract = Contract;
        let message =
            serde_json::to_string(&MTReceiverMode::ReturnValues(vec![U128(10), U128(20)])).unwrap();

        let PromiseOrValue::Value(result) = contract.mt_on_transfer(
            AccountId::from_str("sender.testnet").unwrap(),
            vec![],
            vec!["token1".to_string(), "token2".to_string()],
            vec![U128(100), U128(200)],
            message,
        ) else {
            panic!("expected value promise");
        };

        assert_eq!(result, vec![U128(10), U128(20)]);
    }

    #[test]
    fn mt_on_transfer_panic() {
        let result = std::panic::catch_unwind(|| {
            let mut contract = Contract;
            let message = serde_json::to_string(&MTReceiverMode::Panic).unwrap();

            contract.mt_on_transfer(
                AccountId::from_str("sender.testnet").unwrap(),
                vec![],
                vec!["token".to_string()],
                vec![U128(1)],
                message,
            )
        });

        assert!(result.is_err());
    }

    #[test]
    fn mt_on_transfer_with_large_return() {
        let mut contract = Contract;
        let message = serde_json::to_string(&MTReceiverMode::LargeReturn).unwrap();

        let PromiseOrValue::Value(result) = contract.mt_on_transfer(
            AccountId::from_str("sender.testnet").unwrap(),
            vec![],
            vec!["token".to_string()],
            vec![U128(1)],
            message,
        ) else {
            panic!("expected value promise");
        };

        assert_eq!(result, vec![U128(u128::MAX); 250000]);
    }
}
