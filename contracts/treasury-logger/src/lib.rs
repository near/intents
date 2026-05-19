mod event;
mod state;

use defuse_borsh_utils::adapters::As;
use defuse_nep245::{TokenId, receiver::MultiTokenReceiver};
use impl_tools::autoimpl;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, PromiseOrValue, env, near, require};

use self::{
    event::Event,
    state::{State, VersionedState},
};

#[near(
    contract_state,
    contract_metadata(standard(standard = "logger", version = "0.1.0"))
)]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[derive(Default)]
pub struct Contract(
    #[borsh(
        deserialize_with = "As::<VersionedState>::deserialize",
        serialize_with = "As::<VersionedState>::serialize"
    )]
    State,
);

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
        let token = env::predecessor_account_id();

        require!(
            token != env::current_account_id(),
            "self-deposits are forbidden",
        );
        require!(!amounts.is_empty(), "invalid args");
        require!(
            token_ids.len() == amounts.len(),
            "token_ids and amounts length mismatch"
        );
        require!(
            token_ids.len() == previous_owner_ids.len(),
            "token_ids and previous_owner_ids length mismatch"
        );

        Event::MtDeposit {
            token: token.into(),
            sender_id: sender_id.into(),
            previous_owner_ids: previous_owner_ids.iter().map(Into::into).collect(),
            token_ids: token_ids.iter().map(Into::into).collect(),
            amounts: amounts
                .iter()
                .map(|a| a.0)
                .inspect(|a| require!(*a > 0, "zero amount"))
                .collect(),
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
    use defuse_nep245::receiver::MultiTokenReceiver;
    use near_sdk::{
        AccountId, AccountIdRef,
        json_types::U128,
        test_utils::{VMContextBuilder, get_logs},
        testing_env,
    };
    use rstest::rstest;

    use super::*;

    const CURRENT_ACCOUNT_ID: &AccountIdRef = AccountIdRef::new_or_panic("treasury.near");

    #[rstest]
    #[case(
        "intents.near",
        "alice.near",
        ["alice.near"],
        ["nep141:wrap.near"],
        [1],
        "test"
    )]
    #[case(
        "intents.near",
        "alice.near",
        ["alice.near"],
        ["nep141:wrap.near"],
        [100],
        "test"
    )]
    #[case(
        "intents.near",
        "alice.near",
        ["alice.near"],
        ["nep141:wrap.near"],
        [u128::MAX],
        "test"
    )]
    #[should_panic = "self-deposit"]
    #[case::self_deposit_panics(
        CURRENT_ACCOUNT_ID,
        "alice.near",
        ["alice.near"],
        ["nep141:wrap.near"],
        [100],
        "test"
    )]
    #[should_panic = "zero amount"]
    #[case::zero_amount_panics(
        "intents.near",
        "alice.near",
        ["alice.near"],
        ["nep141:wrap.near"],
        [0],
        "test"
    )]
    fn test_mt_deposit_event<'a>(
        #[case] token: impl AsRef<str>,
        #[case] sender_id: impl AsRef<str>,
        #[case] previous_owner_ids: impl IntoIterator<Item = &'a str>,
        #[case] token_ids: impl IntoIterator<Item = &'a str>,
        #[case] amounts: impl IntoIterator<Item = u128>,
        #[case] msg: &str,
    ) {
        let token: AccountId = token.as_ref().parse().unwrap();
        let sender_id: AccountId = sender_id.as_ref().parse().unwrap();
        let previous_owner_ids: Vec<AccountId> = previous_owner_ids
            .into_iter()
            .map(|a| a.parse().unwrap())
            .collect();

        let token_ids: Vec<_> = token_ids.into_iter().map(ToString::to_string).collect();
        let amounts: Vec<_> = amounts.into_iter().map(U128).collect();

        let context = VMContextBuilder::new()
            .current_account_id(CURRENT_ACCOUNT_ID.into())
            .predecessor_account_id(token.clone())
            .build();
        testing_env!(context);

        let mut contract = Contract::default();
        let _ = contract.mt_on_transfer(
            sender_id.clone(),
            previous_owner_ids.clone(),
            token_ids.clone(),
            amounts.clone(),
            msg.to_string(),
        );

        let actual = get_logs();
        let expected = vec![format!(
            "EVENT_JSON:{}",
            near_sdk::serde_json::json!({
                "standard": "logger",
                "version": "1.0.0",
                "event": "mt_deposit",
                "data": {
                    "token": token,
                    "sender_id": sender_id,
                    "previous_owner_ids": previous_owner_ids,
                    "token_ids": token_ids,
                    "amounts": amounts,
                    "msg": msg,
                    "nonce": "0"
                }
            })
            .to_string()
        )];

        assert_eq!(actual, expected);
        assert_eq!(contract.get_nonce(), U128(1));
    }
}
