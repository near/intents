use std::collections::{BTreeMap, BTreeSet};

use defuse_borsh_utils::adapters::{
    As as BorshAs, TimestampNanoSeconds as BorshTimestampNanoSeconds,
};
use defuse_fees::Pips;
use defuse_token_id::TokenId;
use near_sdk::{AccountId, CryptoHash, Gas, borsh, env, near};
use serde_with::{DisplayFromStr, hex::Hex, serde_as};

use crate::action::{FillAction, TransferAction, TransferMessage};
use crate::{Deadline, Error, Result, decimal::UD128};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractStorage(
    /// If `None`, the escrow was closed and is being deteled now
    pub(crate) Option<Storage>,
);

impl ContractStorage {
    pub(crate) const STATE_KEY: &[u8] = b"";

    #[inline]
    pub fn init(params: &Params) -> Result<Self> {
        Storage::new(params).map(Some).map(Self)
    }

    pub fn init_state(params: &Params) -> Result<BTreeMap<Vec<u8>, Vec<u8>>> {
        let state = Self::init(params)?;
        Ok([(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(&state).map_err(Error::Borsh)?,
        )]
        .into())
    }
}

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

    #[inline]
    pub const fn no_verify(&self) -> &State {
        &self.state
    }

    #[inline]
    pub const fn no_verify_mut(&mut self) -> &mut State {
        &mut self.state
    }

    #[inline]
    pub fn verify(&self, fixed: &Params) -> Result<&State> {
        if self.params_hash != fixed.hash() {
            return Err(Error::InvalidData);
        }
        Ok(&self.state)
    }

    #[inline]
    pub fn verify_mut(&mut self, fixed: &Params) -> Result<&mut State> {
        self.verify(fixed)?;
        Ok(&mut self.state)
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Params {
    pub maker: AccountId,

    pub src_token: TokenId,
    pub dst_token: TokenId, // TODO: one_of

    // TODO: direction? src per 1 dst vs dst per 1 src?
    pub price: UD128, // TODO: dutch auction

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

    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub refund_src_to: OverrideSend,
    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
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
    pub auth_caller: Option<AccountId>,

    #[serde_as(as = "Hex")]
    pub salt: [u8; 32],
}

impl Params {
    #[inline]
    pub fn hash(&self) -> CryptoHash {
        env::keccak256_array(borsh::to_vec(self).unwrap_or_else(|_| unreachable!()))
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
        Ok(())
    }

    fn validate_tokens(&self) -> Result<()> {
        if self.src_token == self.dst_token {
            return Err(Error::SameTokens);
        }
        Ok(())
    }

    const fn validate_price(&self) -> Result<()> {
        if self.price.is_zero() {
            return Err(Error::PriceTooLow);
        }
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
}

/// Builder for creating escrow swap parameters.
/// Takes (maker, `src_token`) and (takers, `dst_token`) tuples associating actors with their tokens.
#[derive(Debug, Clone)]
pub struct ParamsBuilder {
    maker: AccountId,
    src_token: TokenId,
    takers: BTreeSet<AccountId>,
    dst_token: TokenId,
    salt: Option<[u8; 32]>,
    price: Option<crate::decimal::UD128>,
    partial_fills_allowed: Option<bool>,
    deadline: Option<crate::Deadline>,
    fill_deadline: Option<crate::Deadline>,
    refund_src_to: Option<OverrideSend>,
    receive_dst_to: Option<OverrideSend>,
    #[cfg(feature = "auth_call")]
    auth_caller: Option<AccountId>,
    protocol_fees: Option<ProtocolFees>,
    integrator_fees: BTreeMap<AccountId, Pips>,
}

impl ParamsBuilder {
    pub fn new(
        (maker, src_token): (AccountId, TokenId),
        (takers, dst_token): (impl IntoIterator<Item = AccountId>, TokenId),
    ) -> Self {
        Self {
            maker,
            src_token,
            takers: takers.into_iter().collect(),
            dst_token,
            salt: None,
            price: None,
            partial_fills_allowed: None,
            deadline: None,
            fill_deadline: None,
            refund_src_to: None,
            receive_dst_to: None,
            #[cfg(feature = "auth_call")]
            auth_caller: None,
            protocol_fees: None,
            integrator_fees: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn with_salt(mut self, salt: [u8; 32]) -> Self {
        self.salt = Some(salt);
        self
    }

    #[must_use]
    pub const fn with_price(mut self, price: crate::decimal::UD128) -> Self {
        self.price = Some(price);
        self
    }

    #[must_use]
    pub const fn with_partial_fills_allowed(mut self, allowed: bool) -> Self {
        self.partial_fills_allowed = Some(allowed);
        self
    }

    #[must_use]
    pub const fn with_deadline(mut self, deadline: crate::Deadline) -> Self {
        self.deadline = Some(deadline);
        self
    }

    #[must_use]
    pub const fn with_fill_deadline(mut self, fill_deadline: crate::Deadline) -> Self {
        self.fill_deadline = Some(fill_deadline);
        self
    }

    #[must_use]
    pub fn with_refund_src_to(mut self, refund_src_to: OverrideSend) -> Self {
        self.refund_src_to = Some(refund_src_to);
        self
    }

    #[must_use]
    pub fn with_receive_dst_to(mut self, receive_dst_to: OverrideSend) -> Self {
        self.receive_dst_to = Some(receive_dst_to);
        self
    }

    #[cfg(feature = "auth_call")]
    #[must_use]
    pub fn with_auth_caller(mut self, auth_caller: AccountId) -> Self {
        self.auth_caller = Some(auth_caller);
        self
    }

    #[must_use]
    pub fn with_protocol_fees(mut self, protocol_fees: ProtocolFees) -> Self {
        self.protocol_fees = Some(protocol_fees);
        self
    }

    #[must_use]
    pub fn with_integrator_fee(mut self, account_id: AccountId, fee: Pips) -> Self {
        self.integrator_fees.insert(account_id, fee);
        self
    }

    /// Returns the takers whitelist.
    pub const fn takers(&self) -> &BTreeSet<AccountId> {
        &self.takers
    }

    pub fn build(self, default_deadline: Deadline) -> Params {
        Params {
            maker: self.maker,
            src_token: self.src_token,
            dst_token: self.dst_token,
            price: self.price.unwrap_or(UD128::ONE),
            deadline: self.deadline.unwrap_or(default_deadline),
            partial_fills_allowed: self.partial_fills_allowed.unwrap_or(false),
            refund_src_to: self.refund_src_to.unwrap_or_default(),
            receive_dst_to: self.receive_dst_to.unwrap_or_default(),
            taker_whitelist: self.takers,
            protocol_fees: self.protocol_fees,
            integrator_fees: self.integrator_fees,
            #[cfg(feature = "auth_call")]
            auth_caller: self.auth_caller,
            salt: self.salt.unwrap_or([7u8; 32]),
        }
    }

    pub fn build_with_messages(
        self,
        default_deadline: Deadline,
        default_fill_deadline: Deadline,
    ) -> (Params, TransferMessage, TransferMessage) {
        let first_taker = self.takers.first().cloned();
        let fill_deadline = self.fill_deadline.unwrap_or(default_fill_deadline);
        let params = self.build(default_deadline);
        let fund_msg = TransferMessage {
            params: params.clone(),
            action: TransferAction::Fund,
        };
        let fill_msg = TransferMessage {
            params: params.clone(),
            action: TransferAction::Fill(FillAction {
                price: params.price,
                deadline: fill_deadline,
                receive_src_to: first_taker
                    .map(|t| OverrideSend::default().receiver_id(t))
                    .unwrap_or_default(),
            }),
        };
        (params, fund_msg, fill_msg)
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolFees {
    #[serde(default, skip_serializing_if = "Pips::is_zero")]
    pub fee: Pips,
    #[serde(default, skip_serializing_if = "Pips::is_zero")]
    pub surplus: Pips,

    /// NOTE: make sure to have `storage_deposit` for this recipient
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
    /// * no `storage_deposit` for reciipent
    /// * insufficient gas (see `min_gas` below)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gas: Option<Gas>,
}

impl OverrideSend {
    #[must_use]
    pub fn receiver_id(mut self, receiver_id: impl Into<AccountId>) -> Self {
        self.receiver_id = Some(receiver_id.into());
        self
    }

    #[must_use]
    pub fn memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    #[must_use]
    pub fn msg(mut self, msg: impl Into<String>) -> Self {
        self.msg = Some(msg.into());
        self
    }

    #[must_use]
    pub const fn min_gas(mut self, min_gas: Gas) -> Self {
        self.min_gas = Some(min_gas);
        self
    }
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// Funded or lost (after close) src remaining
    #[serde_as(as = "DisplayFromStr")]
    pub maker_src_remaining: u128,

    // Store only lost for maker, since we're bounded in state size
    // So, we don't store lost&found for takers and fee_collectors
    #[serde_as(as = "DisplayFromStr")]
    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub maker_dst_lost: u128,

    #[borsh(
        serialize_with = "BorshAs::<BorshTimestampNanoSeconds>::serialize",
        deserialize_with = "BorshAs::<BorshTimestampNanoSeconds>::deserialize",
        schema(with_funcs(
            declaration = "i64::declaration",
            definitions = "i64::add_definitions_recursively",
        ))
    )]
    pub deadline: Deadline,

    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub closed: bool,

    #[serde(default, skip_serializing_if = "crate::utils::is_default")]
    pub in_flight: u32,
}

// fix JsonSchema macro bug
#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
use near_sdk::serde;
