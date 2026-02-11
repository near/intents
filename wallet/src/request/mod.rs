mod ops;
mod promise;
mod signed;

pub use self::{ops::*, promise::*, signed::*};

use near_sdk::{borsh, env, near};

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    // TODO: accept (optional?) `query_id` and emit an event with it? And/or use `Request` hash
    // as `request_id`?
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ops: Vec<WalletOp>,

    #[serde(default, skip_serializing_if = "PromiseDAG::is_empty")]
    pub out: PromiseDAG,
}

impl Request {
    // TODO: remove or use?
    pub fn hash(&self) -> [u8; 32] {
        env::keccak256_array(borsh::to_vec(self).unwrap_or_else(|_| unreachable!()))
    }
}
