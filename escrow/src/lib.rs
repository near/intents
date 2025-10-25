mod price;

pub use self::price::*;

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use defuse_borsh_utils::adapters::{
    As as BorshAs, TimestampNanoSeconds as BorshTimestampNanoSeconds,
};
use defuse_map_utils::cleanup::DefaultMap;
use defuse_near_utils::{UnwrapOrPanic, UnwrapOrPanicError, time::now};
use defuse_nep245::{ext_mt_core, receiver::MultiTokenReceiver};

use defuse_token_id::nep245::Nep245TokenId as TokenId;
use impl_tools::autoimpl;
use near_sdk::{
    AccountId, FunctionError, NearToken, PanicOnDefault, Promise, PromiseOrValue, PromiseResult,
    assert_one_yocto, env, json_types::U128, near, require, serde_json,
};
use serde_with::{TimestampNanoSeconds as SerdeTimestampNanoSeconds, serde_as};
use thiserror::Error as ThisError;

// mod old;

// TODO: emit logs

// TODO: refund storage_deposits from maker/taker on received tokens
// solution: use intents.near NEP-245

// TODO: coinsidence of wants?

// TODO: custom_resolve()

#[near(contract_state)]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[derive(Debug, PanicOnDefault)]
pub struct Contract(Params);

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
pub struct Params {
    pub maker: AccountId,
    pub maker_asset: TokenId,
    // TODO: remaining_amount?
    pub maker_remaining: u128,

    // TODO: check != maker_asset
    pub taker_asset: TokenId,
    // TODO: multiplier?
    /// maker / taker (in 10^-6)
    pub price: Price,

    // ratio is not a good approach, since adding more maker_tokens
    // would keep the ratio same, rather than decreasing it
    // BUT: maker doesn't want to blindly add assets, he wants to increase to a specific ratio
    // so, he should send maker_asset and forward target_price, the rest will be refunded to him,
    // since there could have been in-flight taker assets coming to the escrow
    // TODO: remaining_amount?
    // pub taker_amount: u128,
    pub taker_asset_receiver_id: AccountId,

    pub partial_fills_allowed: bool,

    // TODO: store only merkle root? leaves have salts
    // pub taker_whitelist: BTreeSet<AccountId>,

    // TODO
    #[borsh(
        serialize_with = "BorshAs::<BorshTimestampNanoSeconds>::serialize",
        deserialize_with = "BorshAs::<BorshTimestampNanoSeconds>::deserialize"
    )]
    #[serde_as(as = "SerdeTimestampNanoSeconds")]
    pub deadline: DateTime<Utc>,
    pub cancel_authority: Option<AccountId>,

    pub lost_found: BTreeMap<AccountId, BTreeMap<TokenId, u128>>,

    pub closed: bool,
    // TODO: keep number of pending promises
}

// escrow     <- one-of-solvers <- solver
//            --------------------> solver
//   (refund) -> one-of-solvers -> solver
// #[near(serializers = [borsh, json])]
// #[derive(Debug)]
// pub enum State {
//     // Just created, no assets received
//     Init,

//     // I.e. received & locked maker asset
//     Open,

//     // TODO: settling what part?
//     // Settling,

//     // TODO
//     Closed,
// }

#[near(serializers = [json])]
#[derive(Debug, Default)]
pub struct MakerMessage {
    pub new_price: Option<Price>,
    // TODO: exact_out support?
}

// #[near]
impl MultiTokenReceiver for Contract {
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        // TODO: utilize for our needs, refund after being closed or expired?
        require!(!self.closed && now() <= self.deadline, "closed");

        let (token_id, amount) = single(token_ids)
            .zip(single(amounts))
            .ok_or(Error::WrongAsset)
            .unwrap_or_panic();

        let asset = TokenId::new(env::predecessor_account_id(), token_id)
            // TODO
            .unwrap();

        // TODO: allow for extended deadline prolongation in msg?
        // TODO: but how can we verify sender_id to allow for that?

        let refund = if asset == self.maker_asset {
            require!(sender_id == self.maker, "unauthorized");

            let msg: MakerMessage = if msg.is_empty() {
                Default::default()
            } else {
                serde_json::from_str(&msg)
                    .unwrap_or_else(|err| env::panic_str(&format!("JSON: {err}")))
            };

            self.maker_remaining = self
                .maker_remaining
                .checked_add(amount.0)
                .ok_or("overflow")
                .unwrap_or_panic_static_str();

            if let Some(new_price) = msg.new_price {
                require!(new_price > self.price, "can't set lower price");
                self.price = new_price;
            }

            0
        } else if asset == self.taker_asset {
            // TODO: taker whitelist
            let msg: TakerMessage = if msg.is_empty() {
                Default::default()
            } else {
                serde_json::from_str(&msg)
                    .unwrap_or_else(|err| env::panic_str(&format!("JSON: {err}")))
            };

            let (maker_amount, taker_amount) = {
                let want_maker_amount = self
                    .price
                    .src_amount(amount.0)
                    .ok_or("overflow")
                    .unwrap_or_panic_static_str();

                // TODO: fees
                if want_maker_amount < self.maker_remaining {
                    require!(self.partial_fills_allowed, "partial fills disallowed");
                    (want_maker_amount, amount.0)
                } else {
                    (
                        self.maker_remaining,
                        self.price
                            .dst_amount(self.maker_remaining)
                            .ok_or("overflow")
                            .unwrap_or_panic_static_str(),
                    )
                }
            };
            self.maker_remaining -= maker_amount;
            let refund = amount.0 - taker_amount;

            // TODO: settle now?
            // TODO: detached send?
            let _ = Self::send(
                self.taker_asset.clone(),
                self.taker_asset_receiver_id.clone(),
                // TODO: add previously failed lost_found amounts?
                taker_amount,
            )
            .and(Self::send(
                self.maker_asset.clone(),
                msg.receiver_id.unwrap_or(sender_id),
                maker_amount,
            ))
            // TODO: optimize and handle lost&found in a single callback
            .then(Self::ext(env::current_account_id()).maybe_cleanup());

            refund
        } else {
            env::panic_str("wrong asset");
        };

        PromiseOrValue::Value(vec![U128(refund)])
    }
}

impl Contract {
    fn send(asset: TokenId, receiver_id: AccountId, amount: u128) -> Promise {
        // TODO: msg for *_transfer_call()?
        let (contract_id, token_id) = asset.clone().into_contract_id_and_mt_token_id();
        ext_mt_core::ext(contract_id)
            .with_attached_deposit(NearToken::from_yoctonear(1))
            // TODO: static gas?
            .mt_transfer(receiver_id.clone(), token_id, U128(amount), None, None)
            .then(
                // TODO: maybe resolve multiple `*_transfer`s at once (i.e. joint promise)
                Self::ext(env::current_account_id())
                    // TODO: static gas?
                    .resolve_transfer(asset, receiver_id, U128(amount)),
            )
    }
}

#[near]
impl Contract {
    #[private]
    pub fn resolve_transfer(
        &mut self,
        asset: TokenId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        let ok = matches!(env::promise_result(0), PromiseResult::Successful(v) if v.is_empty());

        let used = ok.then_some(amount.0).unwrap_or(0);
        let refund = amount.0.saturating_sub(used);
        if refund != 0 {
            let mut receiver = self.lost_found.entry_or_default(receiver_id);
            let mut lost = receiver.entry_or_default(asset);
            *lost = lost
                .checked_add(refund)
                // TODO: is it? no, there can be a malcious token
                .unwrap_or_else(|| unreachable!());
        } else {
            // TODO: maybe cleanup?
        }
        U128(used)
    }

    // permissionless
    // TODO: #[payable] and accept storage_deposits? TODO: set_refund_to()
    // TODO: settle same token in single receipt
    // TODO: return value?
    pub fn lost_found(&mut self, retry: BTreeMap<AccountId, BTreeSet<TokenId>>) {
        // TODO: add support for all-at-once and all-assets-of-receiver-at-once
        for (receiver_id, assets) in retry {
            let mut lost_amounts = self.lost_found.entry_or_default(receiver_id);
            for asset in assets {
                if let Some(lost_amount) = (*lost_amounts).remove(&asset) {
                    // TODO: detach? are we sure
                    let _ = Self::send(asset, lost_amounts.key().clone(), lost_amount);
                }
            }
        }

        // TODO: maybe cleanup?
    }

    #[payable]
    pub fn cancel(&mut self) {
        assert_one_yocto();
        require!(
            self.cancel_authority == Some(env::predecessor_account_id()),
            "unauthorized"
        );

        // TODO: send the rest & delete myself

        // TODO: emit logs

        self.state = State::Closed;
    }
    // TODO: cancel_by_resolver?

    pub fn maybe_cleanup(&mut self) {
        if !self.lost_found.is_empty() {
            return;
        }

        // check pending callbacks counter
        // if self.pending_callbacks != 0 {
        //   return;
        // }

        self.state = State::Closed;
        let _ = Promise::new(env::current_account_id())
            // TODO: are we sure we don't hold any NEAR for storage_deposits, etc..?
            .delete_account(self.taker_asset_receiver_id.clone());
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
    #[error("wrong amount")]
    WrongAmount,
}

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

fn single<T>(v: Vec<T>) -> Option<T> {
    let [a] = v.try_into().ok()?;
    Some(a)
}
