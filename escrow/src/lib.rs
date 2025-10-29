mod price;

pub use self::price::*;

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use defuse_borsh_utils::adapters::{
    As as BorshAs, TimestampNanoSeconds as BorshTimestampNanoSeconds,
};
use defuse_fees::Pips;

use defuse_near_utils::{UnwrapOrPanic, time::now};
use defuse_nep245::{ext_mt_core, receiver::MultiTokenReceiver};
use defuse_num_utils::CheckedAdd;

use defuse_token_id::nep245::Nep245TokenId as TokenId;
use impl_tools::autoimpl;
use near_sdk::{
    AccountId, FunctionError, Gas, NearToken, PanicOnDefault, Promise, PromiseOrValue,
    assert_one_yocto, env, ext_contract, json_types::U128, near, require, serde_json,
};
use serde_with::{DisplayFromStr, TimestampNanoSeconds as SerdeTimestampNanoSeconds, serde_as};
use thiserror::Error as ThisError;

const MT_TRANSFER_GAS: Gas = Gas::from_tgas(15);

#[ext_contract(escrow)]
pub trait Escrow: MultiTokenReceiver {
    fn close(&mut self) -> Promise;
}

// mod old;

// TODO: lost&found?

// TODO: emit logs

// TODO: refund storage_deposits from maker/taker on received tokens
// solution: use intents.near NEP-245

// TODO: coinsidence of wants?

// TODO: custom_resolve()

// ratio is not a good approach, since adding more maker_tokens
// would keep the ratio same, rather than decreasing it
// BUT: maker doesn't want to blindly add assets, he wants to increase to a specific ratio
// so, he should send maker_asset and forward target_price, the rest will be refunded to him,
// since there could have been in-flight taker assets coming to the escrow
// TODO: remaining_amount?
// pub taker_amount: u128,

// TODO: versioned account state?

// TODO: too large state (> ZBA limits)
// solution?: keep hashes of immutable data?
// or maybe even compare with current_account_id?

// TODO: keep number of pending promises
#[near(contract_state)]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[derive(Debug, PanicOnDefault)]
pub struct Contract(State);

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[derive(Debug)]
pub struct State {
    #[serde_as(as = "Hex")]
    pub persistent_state_hash: [u8; 32],

    /// maker / taker (in 10^-9)
    /// TODO: check non-zero
    pub price: Price,

    #[serde_as(as = "DisplayFromStr")]
    pub src_remaining: u128,

    // TODO: check that not expired at create?
    #[borsh(
        serialize_with = "BorshAs::<BorshTimestampNanoSeconds>::serialize",
        deserialize_with = "BorshAs::<BorshTimestampNanoSeconds>::deserialize",
        schema(with_funcs(
            declaration = "i64::declaration",
            definitions = "i64::add_definitions_recursively",
        ))
    )]
    #[serde_as(as = "SerdeTimestampNanoSeconds")]
    pub deadline: DateTime<Utc>,

    // TODO: store only merkle root? leaves have salts
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub taker_whitelist: BTreeSet<AccountId>,
    // TODO: whitelist: Option<signer_id>
    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub closed: bool,
    // TODO: lost_found: store zero for beging transfer, otherwise - fail

    // TODO: recovery() method with 0 src_remaining
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
#[derive(Debug)]
pub struct PersistentState {
    pub maker: AccountId,

    // TODO: nep245: token_id length is less than max on intents.near
    // TODO: check != src_asset
    #[serde_as(as = "DisplayFromStr")]
    pub src_asset: TokenId,
    #[serde_as(as = "DisplayFromStr")]
    pub dst_asset: TokenId,

    // TODO: maker msg
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maker_dst_receiver_id: Option<AccountId>,

    #[serde(default)]
    pub partial_fills_allowed: bool,

    // TODO: check that fees are non-zero
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fees: BTreeMap<AccountId, Pips>,

    // allows:
    //   * price update (solver message: min_price)
    //   * deadline update (short)
    //   * cancel before deadline (longer, shorter)
    // TODO: allow .on_auth()
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maker_authority: Option<AccountId>,
}

#[near(serializers = [json])]
#[derive(Debug, Default)]
pub struct MakerMessage {
    pub new_price: Option<Price>,
    // TODO: exact_out support?
}

#[near]
impl MultiTokenReceiver for Contract {
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        let (token_id, amount) = single(token_ids)
            .zip(single(amounts))
            .ok_or(Error::WrongAsset)
            .unwrap_or_panic();

        let asset = TokenId::new(env::predecessor_account_id(), token_id)
            // TODO
            .unwrap();

        let refund = self
            .on_receive(sender_id, asset, amount.0, &msg)
            .unwrap_or_panic();

        PromiseOrValue::Value(vec![U128(refund)])
    }
}

#[near]
impl Escrow for Contract {
    // TODO: cancel_by_resolver?

    #[payable]
    fn close(&mut self) -> Promise {
        if now() <= self.deadline {
            // TODO: what if swapped everything already?
            // what if more assets are about to arrive?
            require!(
                self.cancel_authority == Some(env::predecessor_account_id()),
                "unauthorized"
            );
            assert_one_yocto();
        }

        require!(!self.closed, "already closed");
        self.closed = true;

        // TODO: ensure src_remaining > 0

        Self::send(
            self.src_asset.clone(),
            // TODO: refund_to?
            self.maker.clone(),
            self.src_remaining,
        )
        .and(Promise::new(env::current_account_id()).delete_account(self.maker.clone()))
    }
}

impl Contract {
    /// Returns refund amount
    fn on_receive(
        &mut self,
        sender_id: AccountId,
        asset: TokenId,
        amount: u128,
        msg: &str,
    ) -> Result<u128> {
        if self.closed || now() > self.deadline {
            // TODO: utilize for our needs, refund after being closed or expired?
            return Err(Error::Closed);
        }

        if asset == self.src_asset {
            return self.on_src_receive(
                sender_id,
                amount,
                if msg.is_empty() {
                    Default::default()
                } else {
                    serde_json::from_str(&msg)?
                },
            );
        }

        if asset != self.dst_asset {
            return Err(Error::WrongAsset);
        }

        self.on_dst_receive(
            sender_id,
            amount,
            if msg.is_empty() {
                Default::default()
            } else {
                serde_json::from_str(&msg)?
            },
        )
    }

    fn on_src_receive(
        &mut self,
        sender_id: AccountId,
        amount: u128,
        msg: MakerMessage,
    ) -> Result<u128> {
        if sender_id != self.maker {
            return Err(Error::Unauthorized);
        }

        self.src_remaining = self
            .src_remaining
            .checked_add(amount)
            .ok_or(Error::IntegerOverflow)?;

        // TODO: allow for extended deadline prolongation in msg?
        // TODO: but how can we verify sender_id to allow for that?

        if let Some(new_price) = msg.new_price {
            if new_price < self.price {
                // TODO: or ignore?
                return Err(Error::LowerPrice);
            }
            self.price = new_price;
        }

        Ok(0)
    }

    fn on_dst_receive(
        &mut self,
        sender_id: AccountId,
        dst_amount: u128,
        msg: TakerMessage,
    ) -> Result<u128> {
        // TODO: taker whitelist

        let (taker_src_amount, mut maker_dst_amount) = {
            let want_src_amount = self
                .price
                .src_amount(dst_amount)
                .ok_or(Error::IntegerOverflow)?;
            // TODO: fees
            if want_src_amount < self.src_remaining {
                if !self.partial_fills_allowed {
                    return Err(Error::PartialFillsNotAllowed);
                }
                (want_src_amount, dst_amount)
            } else {
                (
                    self.src_remaining,
                    self.price
                        // TODO: rounding inside?
                        .dst_amount(self.src_remaining)
                        .ok_or(Error::IntegerOverflow)?,
                )
            }
        };

        // TODO: check taker_src_amount != 0 && maker_dst_amount != 0
        self.src_remaining -= taker_src_amount;
        let refund = dst_amount - maker_dst_amount;

        // send to taker
        let _ = Self::send(
            self.src_asset.clone(),
            msg.receiver_id.unwrap_or(sender_id),
            taker_src_amount,
        );

        // send fees
        for (fee_collector, fee) in &self.fees {
            let fee_amount = fee.fee_ceil(maker_dst_amount);
            maker_dst_amount = maker_dst_amount
                .checked_sub(fee_amount)
                .ok_or(Error::IntegerOverflow)?;
            let _ = Self::send(self.dst_asset.clone(), fee_collector.clone(), fee_amount);
        }

        let _ = Self::send(
            self.dst_asset.clone(),
            self.maker_dst_receiver_id
                .as_ref()
                .unwrap_or(&self.maker)
                .clone(),
            maker_dst_amount,
        );

        Ok(refund)
    }

    fn send(asset: TokenId, receiver_id: AccountId, amount: u128) -> Promise {
        // TODO: msg for *_transfer_call()?
        let (contract_id, token_id) = asset.clone().into_contract_id_and_mt_token_id();
        ext_mt_core::ext(contract_id)
            // TODO: are we sure we have that???
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(MT_TRANSFER_GAS)
            .mt_transfer(receiver_id, token_id, U128(amount), None, None)
    }
}

#[near]
impl Contract {
    #[init]
    pub fn new(params: State) -> Self {
        Self(params)
    }

    pub fn params(&self) -> &State {
        &self.0
    }

    pub fn total_fee(&self) -> Pips {
        self.fees
            .values()
            .copied()
            .try_fold(Pips::ZERO, |total, fee| total.checked_add(fee))
            .ok_or(Error::IntegerOverflow)
            .unwrap_or_panic()
    }

    // TODO
    pub fn effective_price(&self) -> u128 {
        todo!()
    }
}

#[near(serializers = [json])]
#[derive(Debug, Default)]
pub struct TakerMessage {
    pub receiver_id: Option<AccountId>,
}

#[derive(Debug, ThisError, FunctionError)]
pub enum Error {
    #[error("wrong asset")]
    WrongAsset,
    #[error("unauthorized")]
    Unauthorized,
    #[error("integer overflow")]
    IntegerOverflow,
    #[error("can't set to lower price")]
    LowerPrice,
    #[error("partial fills are not allowed")]
    PartialFillsNotAllowed,
    #[error("closed")]
    Closed,
    #[error("JSON: {0}")]
    JSON(#[from] serde_json::Error),
}

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

fn single<T>(v: Vec<T>) -> Option<T> {
    let [a] = v.try_into().ok()?;
    Some(a)
}
