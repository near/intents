use defuse::core::payload::multi::MultiPayload;
use defuse::intents::ext_intents;
use defuse_nep245::{TokenId, receiver::MultiTokenReceiver};
use near_sdk::{
    AccountId, Gas, GasWeight, NearToken, Promise, PromiseOrValue, env, json_types::U128, near,
    serde_json,
};

// Raw extern function to generate and return bytes of specified length
// Input: 8-byte little-endian u64 specifying the length
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn stub_return_bytes() {
    if let Some(input) = near_sdk::env::input() {
        if input.len() >= 8 {
            let len = u64::from_le_bytes(input[..8].try_into().unwrap()) as usize;
            let bytes = vec![0xf0u8; len];
            near_sdk::env::value_return(&bytes);
        }
    }
}

trait ReturnValueExt: Sized {
    fn stub_return_bytes(self, len: u64) -> Self;
}

impl ReturnValueExt for Promise {
    fn stub_return_bytes(self, len: u64) -> Self {
        self.function_call_weight(
            "stub_return_bytes",
            len.to_le_bytes().to_vec(),
            NearToken::ZERO,
            Gas::from_ggas(0),
            GasWeight(1),
        )
    }
}

/// Minimal stub contract used for integration tests.
#[derive(Default)]
#[near(contract_state)]
pub struct Contract;

#[derive(Debug, Clone, Default)]
#[near(serializers = [json])]
#[allow(clippy::large_enum_variant)]
pub enum MTReceiverMode {
    #[default]
    AcceptAll,
    /// Refund all deposited amounts
    RefundAll,
    /// Return u128::MAX for each token (malicious refund attempt)
    MaliciousRefund,
    ReturnValue(U128),
    ReturnValues(Vec<U128>),
    Panic,
    LargeReturn,
    ExecuteAndRefund {
        multipayload: MultiPayload,
        refund_amounts: Vec<U128>,
    },
    /// Return raw bytes of specified length (for testing large return values)
    ReturnBytes(U128),
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
        let _ = sender_id;
        let _ = previous_owner_ids;
        let _ = token_ids;
        let mode = serde_json::from_str(&msg).unwrap_or_default();

        match mode {
            MTReceiverMode::AcceptAll => PromiseOrValue::Value(vec![U128(0); amounts.len()]),
            MTReceiverMode::RefundAll => PromiseOrValue::Value(amounts),
            MTReceiverMode::MaliciousRefund => {
                PromiseOrValue::Value(vec![U128(u128::MAX); amounts.len()])
            }
            MTReceiverMode::ReturnValue(value) => PromiseOrValue::Value(vec![value; amounts.len()]),
            MTReceiverMode::ReturnValues(values) => PromiseOrValue::Value(values),
            MTReceiverMode::Panic => env::panic_str("MTReceiverMode::Panic"),
            // 16 * 250_000 = 4 MB, which is the limit for a contract return value
            MTReceiverMode::LargeReturn => PromiseOrValue::Value(vec![U128(u128::MAX); 250_000]),
            MTReceiverMode::ExecuteAndRefund {
                multipayload,
                refund_amounts,
            } => ext_intents::ext(env::predecessor_account_id())
                .execute_intents(vec![multipayload])
                .then(Self::ext(env::current_account_id()).return_refunds(refund_amounts))
                .into(),
            MTReceiverMode::ReturnBytes(len) => Promise::new(env::current_account_id())
                .stub_return_bytes(len.0.try_into().unwrap())
                .into(),
        }
    }
}

#[near]
impl Contract {
    #[private]
    pub fn return_refunds(&self, refund_amounts: Vec<U128>) -> Vec<U128> {
        refund_amounts
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
