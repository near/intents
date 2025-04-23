use crate::contract::{Contract, ContractExt};
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
                    owner_id: match token_id {
                        defuse_core::tokens::TokenId::Nep141(_account_id) => None,
                        defuse_core::tokens::TokenId::Nep171(account_id, _) => {
                            Some(account_id.clone())
                        }
                        defuse_core::tokens::TokenId::Nep245(_account_id, _) => None,
                    },
                });

        match limit {
            Some(l) => iter.take(l.try_into().unwrap_or_panic_display()).collect(),
            None => iter.collect(),
        }
    }
}
