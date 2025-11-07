use defuse_nep245::{TokenId, receiver::MultiTokenReceiver};
use near_sdk::{AccountId, PromiseOrValue, json_types::U128, near, serde_json};

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
    ExceedGasLimit,
    ExceedLogLimit,
    MaliciousReturn,
}

#[near]
impl MultiTokenReceiver for Contract {
    #[allow(unused_variables)]
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        let mode = serde_json::from_str(&msg).unwrap_or_default();

        match mode {
            MTReceiverMode::ReturnValue(value) => PromiseOrValue::Value(vec![value]),
            MTReceiverMode::AcceptAll => PromiseOrValue::Value(vec![U128(0); amounts.len()]),
            MTReceiverMode::ExceedGasLimit | MTReceiverMode::ExceedLogLimit => {
                panic!("mt_on_transfer invoked with mode: {:?}", mode);
            }
            MTReceiverMode::MaliciousReturn => {
                PromiseOrValue::Value(vec![U128(0xffffffffffffffffffffffffffffffff); 250000])
            }
        }
    }
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
    fn mt_on_transfer_exceeds_gas_limit() {
        let result = std::panic::catch_unwind(|| {
            let mut contract = Contract;
            let message = serde_json::to_string(&MTReceiverMode::ExceedGasLimit).unwrap();

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
    fn mt_on_transfer_exceeds_log_limit() {
        let result = std::panic::catch_unwind(|| {
            let mut contract = Contract;
            let message = serde_json::to_string(&MTReceiverMode::ExceedLogLimit).unwrap();

            contract.mt_on_transfer(
                AccountId::from_str("sender.testnet").unwrap(),
                vec![],
                vec!["token".to_string()],
                vec![U128(1)],
                message,
            );
        });

        assert!(result.is_err());
    }

    #[test]
    fn mt_on_transfer_with_malicious_return() {
        let mut contract = Contract;
        let message = serde_json::to_string(&MTReceiverMode::MaliciousReturn).unwrap();

        let PromiseOrValue::Value(result) = contract.mt_on_transfer(
            AccountId::from_str("sender.testnet").unwrap(),
            vec![],
            vec!["token".to_string()],
            vec![U128(1)],
            message,
        ) else {
            panic!("expected value promise");
        };

        assert_eq!(
            result,
            vec![U128(0xffffffffffffffffffffffffffffffff); 250000]
        );
    }
}
