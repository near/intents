use defuse_nep245::TokenId;
use defuse_nep245::receiver::MultiTokenReceiver;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, PanicOnDefault, PromiseOrValue, env, near, require};

use crate::event::Event;

mod event;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    nonce: u128,
}

#[near]
impl Contract {
    /// Creates a new instance of the contract.
    #[init]
    #[allow(clippy::use_self)]
    pub const fn new() -> Self {
        Self { nonce: 0 }
    }

    /// Returns the current nonce of the contract.
    pub fn get_nonce(&self) -> U128 {
        self.nonce.into()
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
        require!(!amounts.is_empty(), "invalid args");
        require!(
            token_ids.len() == amounts.len(),
            "token_ids and amounts length mismatch"
        );

        let _ = (sender_id, previous_owner_ids);
        let result = vec![U128(0); amounts.len()];
        let token = env::predecessor_account_id();

        Event::MtDeposit {
            token,
            token_ids,
            amounts,
            msg,
            nonce: self.nonce.into(),
        }
        .emit();

        self.inc_nonce();
        PromiseOrValue::Value(result)
    }
}

impl Contract {
    fn inc_nonce(&mut self) {
        self.nonce = self
            .nonce
            .checked_add(1)
            .unwrap_or_else(|| env::panic_str("nonce overflow"));
    }
}

#[cfg(test)]
mod tests {
    use crate::Contract;
    use defuse_nep245::receiver::MultiTokenReceiver;
    use near_sdk::json_types::U128;
    use near_sdk::test_utils::{VMContextBuilder, get_logs};
    use near_sdk::testing_env;

    #[test]
    fn test_mt_deposit_event() {
        let context = VMContextBuilder::new()
            .predecessor_account_id("intents.near".parse().unwrap())
            .build();
        testing_env!(context);

        let mut contract = Contract::new();
        let _ = contract.mt_on_transfer(
            "alice.near".parse().unwrap(),
            vec!["alice.near".parse().unwrap()],
            vec!["nep141:wrap.near".to_string()],
            vec![U128(100)],
            "test".to_string(),
        );

        let actual = get_logs();
        let expected = vec![format!(
            "EVENT_JSON:{}",
            near_sdk::serde_json::json!({
                "standard": "logger",
                "version": "1.0.0",
                "event": "mt_deposit",
                "data": {
                    "token": "intents.near",
                    "token_ids": ["nep141:wrap.near"],
                    "amounts": ["100"],
                    "msg": "test",
                    "nonce": "0"
                }
            })
            .to_string()
        )];

        assert_eq!(actual, expected);
        assert_eq!(contract.get_nonce(), U128(1));
    }
}
