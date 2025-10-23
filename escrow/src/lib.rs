use std::{collections::BTreeMap, iter};

use chrono::{DateTime, Utc};
use defuse_borsh_utils::adapters::{
    As as BorshAs, TimestampNanoSeconds as BorshTimestampNanoSeconds,
};
use defuse_map_utils::cleanup::DefaultMap;
use defuse_near_utils::{UnwrapOrPanic, time::now};
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

// TODO: refund storage_deposits from maker/taker on received tokens
// solution: use intents.near NEP-245

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
    pub maker_asset: TokenId,
    pub maker_amount: u128,

    pub taker_asset: TokenId,
    // ratio is not a good approach, since adding more maker_tokens
    // would keep the ratio same, rather than decreasing it
    pub taker_amount: u128,

    pub taker_asset_receiver_id: AccountId,

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
    pub state: State,
    // TODO: keep number of pending promises
}

impl Params {
    pub fn is_alive(&self) -> bool {
        !matches!(self.state, State::Cancelled) && self.deadline > now()
    }
}

// escrow     <- one-of-solvers <- solver
//            --------------------> solver
//   (refund) -> one-of-solvers -> solver

#[near(serializers = [borsh, json])]
#[derive(Debug)]
pub enum State {
    // Just created, no assets received
    Init,

    // I.e. received & locked maker asset
    Open,

    // TODO: settling what part?
    // Settling,

    // TODO
    Cancelled,
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

        require!(self.is_alive(), "expired or cancelled");

        // TODO: allow for extended deadline prolongation in msg?
        // TODO: but how can we verify sender_id to allow for that?

        match self.state {
            State::Init => {
                require!(asset == self.maker_asset, "wrong asset");
                // TODO: verify sender_id?
                require!(amount.0 >= self.maker_amount, "insufficient amount");
                // TODO: are we sure that we want to update amount?
                self.maker_amount = amount.0;

                self.state = State::Open;
                return PromiseOrValue::Value(vec![U128(0)]);
            }
            State::Open => {
                if asset == self.maker_asset {
                    // TODO: add support for increase maker asset amount
                    self.maker_amount += amount.0;
                    return PromiseOrValue::Value(vec![U128(0)]);
                }
                require!(asset == self.taker_asset, "wrong asset");
                // TODO: partial fills
                let refund = amount
                    .0
                    .checked_sub(self.taker_amount)
                    .ok_or("insufficient amount")
                    .unwrap_or_panic();

                let msg: TakerMessage = serde_json::from_str(&msg).unwrap();
                let maker_asset_receiver_id = msg.receiver_id.unwrap_or(sender_id);

                // TODO: self.filled += ...?

                // TODO: settle now?
                // detached send
                let _ = Self::send(
                    self.taker_asset.clone(),
                    self.taker_asset_receiver_id.clone(),
                    // TODO: add previously failed lost_found amounts?
                    self.taker_amount,
                )
                .and(Self::send(
                    self.maker_asset.clone(),
                    maker_asset_receiver_id.clone(),
                    self.maker_amount,
                ))
                .then(Self::ext(env::current_account_id()).maybe_cleanup());

                PromiseOrValue::Value(vec![U128(refund)])
            }
            State::Cancelled => unreachable!(), // TODO
        }
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
                // TODO: is it?
                .unwrap_or_else(|| unreachable!());
        }
        U128(used)
    }

    // permissionless
    // TODO: return value?
    pub fn lost_found(&mut self, retry: BTreeMap<AccountId, BTreeMap<TokenId, U128>>) {
        for (receiver_id, (asset, amount)) in retry
            .into_iter()
            .flat_map(|(receiver_id, amounts)| iter::repeat(receiver_id).zip(amounts))
        {
            // TODO: detach? are we sure
            let _ = Self::send(asset, receiver_id, amount.0);
        }
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

        self.state = State::Cancelled;
    }

    pub fn maybe_cleanup(self) {
        // self
    }
}

#[near(serializers = [json])]
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
