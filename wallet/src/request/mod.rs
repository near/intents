mod ops;
mod promise;
mod signed;

pub use self::{ops::*, promise::*, signed::*};

use near_sdk::{borsh, env, near};

// TODO: versioned? or support versioned via different contract methods?
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    // TODO: query_id? + emit?
    // TODO: what about extensions? maybe make query_id optional?
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ops: Vec<WalletOp>,

    #[serde(default, skip_serializing_if = "PromiseDAG::is_empty")]
    pub out: PromiseDAG,
    // TODO: can it be implemented as additional methods?
    // TODO: use-case: other wallet asks "verify this signature
    // with arbitrary data, I'll do the verification of the data"
    // TODO: this can be achieved by using oneshot-condvar contract:
    // Custom wallet-specific operations
    // #[cfg_attr(
    //     all(feature = "abi", not(target_arch = "wasm32")),
    //     schemars(with = "String")
    // )]
    // #[serde_as(as = "Option<Base64>")]
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub custom: Option<Vec<u8>>,
}

impl Request {
    pub fn hash(&self) -> [u8; 32] {
        env::keccak256_array(borsh::to_vec(self).unwrap_or_else(|_| unreachable!()))
    }
}
