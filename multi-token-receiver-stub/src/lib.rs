use defuse_nep245::{TokenId, receiver::MultiTokenReceiver};
use near_sdk::{
    AccountId, PromiseOrValue,
    env::sha256,
    json_types::U128,
    log, near,
    serde::{Deserialize, Serialize},
    serde_json,
};

/// Minimal stub contract used for integration tests.
#[derive(Default)]
#[near(contract_state)]
pub struct Contract;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum MTReceiverMode {
    AcceptAll,
    ReturnValue(U128),
    ExceedGasLimit,
    ExceedLogLimit,
}

impl MTReceiverMode {
    fn decode(msg: &str) -> Self {
        serde_json::from_str(msg)
            .unwrap_or_else(|err| panic!("failed to deserialize MTReceiverMode: {err}"))
    }

    #[cfg(test)]
    fn encode(&self) -> String {
        serde_json::to_string(self).expect("MTReceiverMode::encode serialization should succeed")
    }
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
        match MTReceiverMode::decode(&msg) {
            MTReceiverMode::ReturnValue(value) => return PromiseOrValue::Value(vec![value]),
            MTReceiverMode::AcceptAll => {
                return PromiseOrValue::Value(vec![U128(0); amounts.len()]);
            }
            MTReceiverMode::ExceedGasLimit => {
                for i in 0..100 {
                    sha256(i.to_string().as_bytes());
                }
            }
            MTReceiverMode::ExceedLogLimit => {
                for _ in 0..100 {
                    log!(
                        "NEAR Intents is a multichain transaction protocol where users specify what they want and let third parties compete to provide the best solution. This works for everything from token swaps to pizza delivery, creating a universal marketplace across crypto and traditional services."
                    );
                }
            }
        }

        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use near_sdk::test_utils::VMContextBuilder;

    use super::*;

    #[test]
    fn mt_on_transfer_returns_requested_value() {
        let mut contract = Contract;
        let message = MTReceiverMode::ReturnValue(U128(42)).encode();

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
        let message = MTReceiverMode::AcceptAll.encode();

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
        let mut builder = VMContextBuilder::new();
        builder.prepaid_gas(near_sdk::Gas::from_gas(1));
        near_sdk::testing_env!(builder.build());

        let result = std::panic::catch_unwind(|| {
            let mut contract = Contract;
            let message = MTReceiverMode::ExceedGasLimit.encode();

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
            let message = MTReceiverMode::ExceedLogLimit.encode();

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
}
