//TODO: remove once escrow is merged

use defuse_deadline::Deadline;
use defuse_fees::Pips;
use defuse_token_id::TokenId;
use near_sdk::{borsh, CryptoHash, Gas};
use near_sdk::{AccountId, json_types::U128, near};
use std::collections::{BTreeMap, BTreeSet};

use serde_with::hex::Hex;

use defuse_borsh_utils::adapters::{
    As as BorshAs, TimestampNanoSeconds as BorshTimestampNanoSeconds,
};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolFees {
    #[serde(default, skip_serializing_if = "Pips::is_zero")]
    pub fee: Pips,
    #[serde(default, skip_serializing_if = "Pips::is_zero")]
    pub surplus: Pips,

    /// NOTE: make sure to have `storage_deposit` for this recepient
    /// on `dst_token`
    pub collector: AccountId,
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverrideSend {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver_id: Option<AccountId>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// NOTE: No refund will be made in case of
    /// `*_transfer_call()` failed. Reasons for it to fail:
    /// * no `storage_deposit` for receipent
    /// * insufficient gas (see `min_gas` below)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gas: Option<Gas>,
}

impl OverrideSend {
    pub fn receiver_id(mut self, receiver_id: impl Into<AccountId>) -> Self {
        self.receiver_id = Some(receiver_id.into());
        self
    }

    pub fn memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    pub fn msg(mut self, msg: impl Into<String>) -> Self {
        self.msg = Some(msg.into());
        self
    }

    pub fn min_gas(mut self, min_gas: Gas) -> Self {
        self.min_gas = Some(min_gas);
        self
    }
}

#[inline]
pub fn is_default<T>(v: &T) -> bool
where
    T: Default + PartialEq,
{
    *v == T::default()
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Params {
    pub maker: AccountId,

    pub src_token: TokenId,
    pub dst_token: TokenId, // TODO: one_of

    pub price: U128, // TODO: dutch auction

    #[borsh(
        serialize_with = "BorshAs::<BorshTimestampNanoSeconds>::serialize",
        deserialize_with = "BorshAs::<BorshTimestampNanoSeconds>::deserialize",
        schema(with_funcs(
            declaration = "i64::declaration",
            definitions = "i64::add_definitions_recursively",
        ))
    )]
    pub deadline: Deadline,

    #[serde(default)]
    pub partial_fills_allowed: bool,

    #[serde(default, skip_serializing_if = "is_default")]
    pub refund_src_to: OverrideSend,
    #[serde(default, skip_serializing_if = "is_default")]
    pub receive_dst_to: OverrideSend,

    // taker_whitelist: ["solver-bus-proxy.near"] (knows SolverBus public key)
    //
    // solver -> intents.near::mt_transfer_call():
    //   * solver-bus-proxy.near::mt_on_transfer(sender_id, token_id, amount, msg):
    //      msg.extract_solver_bus_signature()
    //               verify_signature()
    //               if ok -> forward transfer to escrow specified in msg
    //               if not ok -> refund solver
    //
    // solver-bus.near -> solver-bus-proxy.near::close(escrow_contract_id)
    //                 -> escrow-0x1234....abc::close()
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub taker_whitelist: BTreeSet<AccountId>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_fees: Option<ProtocolFees>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub integrator_fees: BTreeMap<AccountId, Pips>,

    #[cfg(feature = "auth_call")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_caller: Option<AccountId>, // TODO: or parent account id?

    // #[serde_as(as = "Hex")]
    pub salt: [u8; 32],
}

impl Params {
    fn hash(&self) -> CryptoHash {
        // TODO: prefix?
        near_sdk::env::keccak256_array(borsh::to_vec(self).unwrap_or_else(|_| unreachable!()))
    }
}
