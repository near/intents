use crate::contract::{Contract, ContractExt};
use defuse_core::tokens::TokenIdType;
use defuse_near_utils::UnwrapOrPanicError;
use defuse_nep245::{Token, enumeration::MultiTokenEnumeration};
use near_sdk::{AccountId, json_types::U128, near};

#[near]
impl MultiTokenEnumeration for Contract {
    fn mt_tokens(&self, from_index: Option<U128>, limit: Option<u32>) -> Vec<Token> {
        let from_index = from_index.map_or(0, |v| v.0);
        let from_index: usize = from_index.try_into().unwrap_or_panic_display();

        let iter = self
            .state
            .total_supplies
            .iter()
            .skip(from_index)
            .map(|(token_id, _amount)| Token {
                token_id: token_id.to_string(),
                // Note: There is no way to fill this field currently (which is required for NEP-171/NFTs),
                // as it requires reverse look-up for tokens and that's expensive for storage.
                // We will postpone this decision for the future when it's needed.
                owner_id: None,
            });

        match limit {
            Some(l) => iter.take(l.try_into().unwrap_or_panic_display()).collect(),
            None => iter.collect(),
        }
    }

    fn mt_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u32>,
    ) -> Vec<Token> {
        let from_index = from_index.map_or(0, |v| v.0);
        let from_index: usize = from_index.try_into().unwrap_or_panic_display();

        let Some(account) = self.accounts.get(&account_id) else {
            return Vec::new();
        };

        let iter =
            account
                .state
                .token_balances
                .iter()
                .skip(from_index)
                .map(|(token_id, _amount)| Token {
                    token_id: token_id.to_string(),
                    owner_id: match token_id.into() {
                        TokenIdType::Nep171 => Some(account_id.clone()),
                        TokenIdType::Nep141 | TokenIdType::Nep245 => None,
                    },
                });

        match limit {
            Some(l) => iter.take(l.try_into().unwrap_or_panic_display()).collect(),
            None => iter.collect(),
        }
    }
}
