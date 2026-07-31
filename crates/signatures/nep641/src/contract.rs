use std::collections::HashMap;

use near_account_id::AccountId;

use crate::{OffchainMessage, Proof};

/// A smart-contract implementing NEP-641 interface.
pub trait AuthResolver {
    /// A view-method to resolve [offchain](#offchain-only) authorization
    /// according to NEP-641.
    ///
    /// The implementation MUST verify given `proof` over offchain message, including
    /// all of its fields, and return a list of "pending authorizations" that need to be
    /// [resolved](crate::resolver::OffchainResolver::resolve_auth) on other accounts.
    ///
    /// TODO: returned auths fields
    ///
    /// The implementation MUST panic if:
    /// * [`chain_id`](field@crate::OffchainMessage::chain_id) doesn't match
    ///   `env::chain_id()`
    /// * [`resolver_id`](field@crate::OffchainMessage::resolver_id) doesn't
    ///   match `env::current_account_id()`
    /// * `proof` is invalid for given [`OffchainMessage`](crate::OffchainMessage)
    ///
    /// # Offchain Only
    ///
    /// **DO NOT** call this view-method in on-chain transactions!
    ///
    /// NEP-641 standard is **not** intended to be used for on-chain transfer "approvals"
    /// or any other actions that modify state of the blockchain. Offchain messages are
    /// intended to be verified _only_ offchain as they doesn't mutate any state on the
    /// [resolver](field@crate::OffchainMessage::resolver_id)'s account and, hence, cannot
    /// prevent replay attacks.
    ///
    /// Instead, use on-chain messages (e.g. request messages, transactions, delegate actions),
    /// which are specifically designed with replay-protection mechanism in mind.
    // TODO: can we resolve (different?) signatures on same account id multiple
    // times? e.g. intents.near
    fn w_resolve_auth(&self, msg: OffchainMessage, proof: Proof) -> HashMap<AccountId, Proof>;
}
