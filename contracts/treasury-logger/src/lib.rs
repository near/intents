use defuse_nep245::TokenId;
use defuse_nep245::receiver::MultiTokenReceiver;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, PromiseOrValue, env, near, require};

use crate::event::Event;

mod event;

#[near(
    contract_state,
    contract_metadata(standard(standard = "logger", version = "0.1.0"),)
)]
#[derive(Default)]
pub struct Contract {
    nonce: u128,
}

#[near]
impl Contract {
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
        require!(
            token_ids.len() == previous_owner_ids.len(),
            "previous_owner_ids and amounts mismatch"
        );

        Event::MtDeposit {
            token: env::predecessor_account_id().into(),
            sender_id: sender_id.into(),
            previous_owner_ids: previous_owner_ids.iter().map(Into::into).collect(),
            token_ids: token_ids.iter().map(Into::into).collect(),
            amounts: amounts.iter().map(|a| a.0).collect(),
            msg: msg.into(),
            nonce: self.next_nonce(),
        }
        .emit();

        PromiseOrValue::Value(vec![U128(0); amounts.len()])
    }
}

impl Contract {
    #[must_use]
    fn next_nonce(&mut self) -> u128 {
        let nonce = self.nonce;
        self.nonce = self
            .nonce
            .checked_add(1)
            .unwrap_or_else(|| env::panic_str("nonce overflow"));
        nonce
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

        let mut contract = Contract::default();
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
                    "sender_id": "alice.near",
                    "previous_owner_ids": ["alice.near"],
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
