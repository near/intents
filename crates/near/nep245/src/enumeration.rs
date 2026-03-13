use crate::Token;
use near_sdk::{AccountId, ext_contract, json_types::U128};

/// A trait representing the [multi-token enumeration standard](https://nomicon.io/Standards/Tokens/MultiToken/Enumeration#interface).
#[ext_contract(ext_mt_enumeration)]
pub trait MultiTokenEnumeration {
    /// Get a list of all tokens
    ///
    /// Arguments:
    /// * `from_index`: a string representing an unsigned 128-bit integer,
    ///    representing the starting index of tokens to return
    /// * `limit`: the maximum number of tokens to return
    ///
    /// Returns an array of `Token` objects, as described in the Core standard,
    /// and an empty array if there are no tokens
    fn mt_tokens(&self, from_index: Option<U128>, limit: Option<u32>) -> Vec<Token>;

    /// Get list of all tokens owned by a given account
    ///
    /// Arguments:
    /// * `account_id`: a valid NEAR account
    /// * `from_index`: a string representing an unsigned 128-bit integer,
    ///    representing the starting index of tokens to return
    /// * `limit`: the maximum number of tokens to return
    ///
    /// Returns a paginated list of all tokens owned by this account, and an empty array if there are no tokens
    fn mt_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u32>,
    ) -> Vec<Token>;
}
