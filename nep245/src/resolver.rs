use near_sdk::{AccountId, json_types::U128};

use super::{ClearedApproval, TokenId};

pub trait MultiTokenResolver {
    fn mt_resolve_transfer(
        &mut self,
        previous_owner_ids: Vec<AccountId>,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<Vec<ClearedApproval>>>>,
    ) -> Vec<U128>;
}
