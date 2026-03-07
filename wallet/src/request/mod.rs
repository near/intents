mod ops;
mod promise;

pub use self::{ops::*, promise::*};

use near_sdk::near;

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ops: Vec<WalletOp>,

    #[serde(default, skip_serializing_if = "PromiseDAG::is_empty")]
    pub out: PromiseDAG,
}
