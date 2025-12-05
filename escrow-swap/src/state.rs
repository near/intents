use std::collections::{BTreeMap, BTreeSet};

use defuse_borsh_utils::adapters::{
    As as BorshAs, TimestampNanoSeconds as BorshTimestampNanoSeconds,
};
use defuse_fees::Pips;
use defuse_token_id::TokenId;
use near_sdk::{AccountId, AccountIdRef, CryptoHash, Gas, borsh, env, near};
use serde_with::{DisplayFromStr, hex::Hex, serde_as};

use crate::{Deadline, Error, Result, price::Price};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractStorage(pub(crate) Option<Storage>);

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

    pub price: Price, // TODO: dutch auction

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
    pub auth_caller: Option<AccountId>, // TODO: or parent account id?

    #[serde_as(as = "Hex")]
    pub salt: [u8; 32],
}

impl Params {
    #[inline]
    pub fn hash(&self) -> CryptoHash {
        // TODO: prefix?
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
