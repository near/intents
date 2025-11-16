use std::collections::{BTreeMap, BTreeSet};

use defuse_borsh_utils::adapters::{
    As as BorshAs, TimestampNanoSeconds as BorshTimestampNanoSeconds,
};
use defuse_fees::Pips;
use defuse_near_utils::time::Deadline;
use defuse_num_utils::CheckedAdd;
use defuse_token_id::TokenId;
use near_sdk::{AccountId, AccountIdRef, CryptoHash, Gas, borsh, env, near};
use serde_with::{DisplayFromStr, hex::Hex, serde_as};

use crate::{
    Error, Result,
    price::Price,
    tokens::{OverrideSend, TokenIdExt},
};

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]

pub struct Storage {
    #[serde_as(as = "Hex")]
    params_hash: CryptoHash,

    #[serde(flatten)]
    state: State,
}

impl Storage {
    pub fn new(params: &Params) -> Result<Self> {
        params.validate()?;

        Ok(Self {
            params_hash: params.hash(),
            state: State {
                maker_src_remaining: 0,
                maker_dst_lost: 0,
                deadline: params.deadline,
                closed: false,
                in_flight: 0,
            },
        })
    }

    // TODO: nep616 feature
    pub fn derive_account_id(&self, factory: impl AsRef<AccountIdRef>) -> AccountId {
        // TODO: remove
        const PREFIX: &str = "escrow-";

        let factory = factory.as_ref();

        let serialized = borsh::to_vec(self).unwrap_or_else(|_| unreachable!());
        let hash = env::keccak256_array(&serialized);

        let len = AccountId::MAX_LEN - 1 - factory.len() - PREFIX.len();
        format!("{PREFIX}{}.{factory}", hex::encode(&hash[32 - len / 2..32]))
            .parse()
            .unwrap_or_else(|_| unreachable!())
    }

    pub const fn no_verify(&self) -> &State {
        &self.state
    }

    pub const fn no_verify_mut(&mut self) -> &mut State {
        &mut self.state
    }

    pub fn verify(&self, fixed: &Params) -> Result<&State> {
        (self.params_hash == fixed.hash())
            .then_some(&self.state)
            .ok_or(Error::InvalidData)
    }

    pub fn verify_mut(&mut self, fixed: &Params) -> Result<&mut State> {
        self.verify(fixed)?;
        Ok(&mut self.state)
    }
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Params {
    pub maker: AccountId,

    // TODO: support one_of for dst
    pub src_token: TokenId, // in case of loan: dst_asset
    pub dst_token: TokenId, // in case of loan:

    /// maker / taker (in 10^-9)
    /// TODO: check non-zero
    /// TODO: dutch auction
    pub price: Price,

    // TODO: check that not expired at create?
    #[borsh(
        serialize_with = "BorshAs::<BorshTimestampNanoSeconds>::serialize",
        deserialize_with = "BorshAs::<BorshTimestampNanoSeconds>::deserialize",
        schema(with_funcs(
            declaration = "i64::declaration",
            definitions = "i64::add_definitions_recursively",
        ))
    )]
    // #[serde_as(as = "SerdeTimestampNanoSeconds")] // TODO: RFC-3339
    pub deadline: Deadline,

    #[serde(default)]
    pub partial_fills_allowed: bool,

    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub refund_src_to: OverrideSend,
    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub receive_dst_to: OverrideSend,

    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub taker_whitelist: BTreeSet<AccountId>,

    // taker_whitelist: ["solver-bus-proxy.near"] (knows SolverBus public key)

    // solver -> intents.near::mt_transfer_call():
    //   * solver-bus-proxy.near::mt_on_transfer(sender_id, token_id, amount, msg):
    //      msg.extract_solver_bus_signature()
    //               verify_signature()
    //               if ok -> forward transfer to escrow specified in msg
    //               if not ok -> refund solver
    //
    // solver-bus.near -> solver-bus-proxy.near::close(escrow_contract_id)
    //                 -> escrow-0x1234....abc::close()
    pub protocol_fees: Option<ProtocolFees>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub integrator_fees: BTreeMap<AccountId, Pips>,

    // TODO: or parent account id?
    #[cfg(feature = "auth_call")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_caller: Option<AccountId>,

    #[serde_as(as = "Hex")]
    pub salt: [u8; 32],
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolFees {
    #[serde(default, skip_serializing_if = "Pips::is_zero")]
    fee: Pips,
    #[serde(default, skip_serializing_if = "Pips::is_zero")]
    surplus: Pips,
    collector: AccountId,
}

impl Params {
    pub fn hash(&self) -> CryptoHash {
        // TODO: prefix?
        env::keccak256_array(&borsh::to_vec(self).unwrap_or_else(|_| unreachable!()))
    }

    pub fn total_fee(&self) -> Option<Pips> {
        self.integrator_fees
            .values()
            .copied()
            .try_fold(Pips::ZERO, Pips::checked_add)
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_tokens()?;
        self.validate_price()?;
        self.validate_fees()?;
        self.validate_gas()?;
        Ok(())
    }

    fn validate_tokens(&self) -> Result<()> {
        (self.src_token != self.dst_token)
            .then_some(())
            .ok_or(Error::SameTokens)
    }

    fn validate_price(&self) -> Result<()> {
        // TODO: non zero
        Ok(())
    }

    fn validate_fees(&self) -> Result<()> {
        const MAX_FEE_PERCENT: u32 = 25;
        const MAX_FEE: Pips = Pips::ONE_PERCENT.checked_mul(MAX_FEE_PERCENT).unwrap();

        self.total_fee()
            .is_some_and(|total| total <= MAX_FEE)
            .then_some(())
            .ok_or(Error::ExcessiveFees)
    }

    fn validate_gas(&self) -> Result<()> {
        // mt_on_transfer() with p256 signature validation
        const MAX_FILL_GAS: Gas = Gas::from_tgas(300 - 30 - 10);

        self.required_gas_to_fill()
            .is_some_and(|total| total <= MAX_FILL_GAS)
            .then_some(())
            .ok_or(Error::ExcessiveGas)
    }

    fn required_gas_to_fill(&self) -> Option<Gas> {
        const FILL_GAS: Gas = Gas::from_tgas(20);

        FILL_GAS
            .checked_add(self.dst_token.transfer_gas(
                self.receive_dst_to.min_gas,
                self.receive_dst_to.msg.is_some(),
            ))?
            .checked_add(
                self.src_token
                    .transfer_gas(self.refund_src_to.min_gas, self.refund_src_to.msg.is_some()),
            )?
            .checked_add(
                self.dst_token.transfer_gas(None, false).checked_mul(
                    self.integrator_fees
                        .values()
                        .copied()
                        .filter(|fee| !fee.is_zero())
                        .count()
                        .try_into()
                        .unwrap_or_else(|_| unreachable!()),
                )?,
            )
    }
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// Deposited or lost (after close) src remaining
    #[serde_as(as = "DisplayFromStr")]
    pub maker_src_remaining: u128,

    // Store only lost for maker, since we're bounded in state size
    // So, we don't store lost&found for takers and fee_collectors
    #[serde_as(as = "DisplayFromStr")]
    pub maker_dst_lost: u128,

    // TODO: check that not expired at create?
    #[borsh(
        serialize_with = "BorshAs::<BorshTimestampNanoSeconds>::serialize",
        deserialize_with = "BorshAs::<BorshTimestampNanoSeconds>::deserialize",
        schema(with_funcs(
            declaration = "i64::declaration",
            definitions = "i64::add_definitions_recursively",
        ))
    )]
    // #[serde_as(as = "SerdeTimestampNanoSeconds")] // TODO: RFC-3339
    pub deadline: Deadline,

    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub closed: bool,

    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub in_flight: u32,
}
