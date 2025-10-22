use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use defuse_borsh_utils::adapters::{
    As as BorshAs, TimestampNanoSeconds as BorshTimestampNanoSeconds,
};
use defuse_near_utils::UnwrapOrPanicError;
use impl_tools::autoimpl;
use near_contract_standards::fungible_token::{core::ext_ft_core, receiver::FungibleTokenReceiver};
use near_sdk::{
    AccountId, AccountIdRef, NearToken, PanicOnDefault, Promise, PromiseOrValue, PromiseResult,
    env, json_types::U128, near, require, serde_json,
};
use serde_with::{TimestampNanoSeconds as SerdeTimestampNanoSeconds, serde_as};

// QUESTIONS:
// * settle every time via `mt_transfer()`? if not, i.e. accumulate and send as batch, then:
//   * what do we do with deadline: what if not expired yet but filled 99%?
//   * why to pay for gas when it could have been done by solvers and embedded into the price?
//   *
// * cancel by 2-of-2 multisig: user + SolverBus?
//   * why not 1-of-2 by SolverBus?
//

// governor: partial release

// No `ft_transfer_call()` reasoning:
// * retries with same `msg`
// * NEP-141 vulnerability makes it possible to lose funds if no storage_deposit
// * somethimes logic of ft_transfer_call can be so hard, that it requires additional
//   storage_deposits on THE RECEIVER of the tokens, not only for token itself (e.g. omni-bridge)
// * everything can (and should??) be implemented via off-chain indexers and relayers fo finalize any custom logic
// TODO: add support for custom ".on_settled()" hooks?

// TODO: streaming swaps:
// * cancel of long streaming swaps?
// solution: time-lock (i.e. "delayed" canceling)
// + solver can confirm that he acknoliged the cancel, so it's a multisig 2-of-2 for immediate cancellation

// TODO: partial fills:
// * allow_partial_fills: bool
// * memo/msg:
//   * send as batch all at once?
//   * if we send only when full fill was reached, then it disincentivize the last solver to give full amount, so he can just send 1 unit less
//      * can we incentivize by releasing some locked NEAR?
//   * if it's a user-only claims, then this ruins UX: we need to ask user to
//     claim once the funds are available
//   * if we allow third-party to claim, then there are two approaches:
//      * permissioned: only trusted account can claim funds with propper msg
//         * or a per-claim smart-contract?
//      * permissionless:
#[near(contract_state, serializers = [borsh, json])]
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
    // TODO: cancel: who is able?
    pub maker_id: AccountId,

    pub maker_token_id: AccountId,
    pub maker_amount: u128,

    pub taker_token_id: AccountId,
    // TODO: or ratio? seems like to be needed only for partial fills
    pub taker_amount: u128,
    pub taker_whitelist: BTreeSet<AccountId>,

    pub receiver_id: Option<AccountId>, // maker_id otherwise
    pub receiver_memo: Option<String>,
    // TODO: receiver_msg only if nep141, for sFTs state init might be needed
    pub receiver_msg: Option<String>,

    // TODO: can it then be a deteministic contract supporting multisig or any-of functionality
    pub cancel_authority: Option<AccountId>,

    pub state: State,
    // TODO: what if only partially filled when deadline expires?
    // * is it safe to send funds via msg?
    // #[borsh(
    //     skip,
    //     // serialize_with = "As::<TimestampNanoSeconds>::serialize",
    //     // deserialize_with = "As::<TimestampNanoSeconds>::deserialize"
    // )]
    // #[serde_as(as = "SerdeTimestampNanoSeconds")]
    // pub deadline: DateTime<Utc>,
    // pub salt: [u8; 4], // TODO: only for NEP-616
    // TODO: fees:
    // * hard-code protocol fee_collector
    // * app fees
}

// TODO: maker_asset cannot have msg, since it will be set by taker
#[near(serializers = [borsh, json])]
// TODO: serde tag
pub enum TakerAsset {
    Nep141 {
        contract_id: AccountId,
        amount: u128,

        receiver_id: AccountId,
        memo: Option<String>,
        msg: Option<String>,
    },
    Nep171 {
        contract_id: AccountId,
        token_id: String,

        receiver_id: AccountId,
        memo: Option<String>,
        msg: Option<String>,
    },
    Nep245 {
        contract_id: AccountId,
        token_id: String,
        amount: u128,

        receiver_id: AccountId,
        memo: Option<String>,
        msg: Option<String>,
    },
    // TODO: custom_resolve / governor?
}

impl Params {
    #[inline]
    pub fn taker_asset_receiver_id(&self) -> &AccountId {
        self.receiver_id.as_ref().unwrap_or(&self.maker_id)
    }
}

// TODO
pub struct PartialFillsParams {
    pub claim_manually: bool,
    pub solvers_whitelist: BTreeSet<AccountId>, // Or IterableSet?
}

#[near(serializers = [borsh, json])]
#[derive(Debug)]
pub enum State {
    // Just initialized
    Init,
    /// I.e. received & locked maker asset
    Open,

    Settling,

    // TODO: settling
    LostFound {
        taker_asset_lost: u128,
        maker_asset_lost: u128,
    },
}

#[near(serializers = [json])]
pub struct TakerMessage {
    // TODO: Option, or sender_id of ft_on_transfer
    pub receiver_id: AccountId,
    pub memo: Option<String>,
    pub msg: Option<String>,
    // TODO: storage_deposit???
    // TODO: min_gas
}

#[near]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        // it could have been EscrowFactory who forwarded funds to us
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        // TODO: verify self.taker_asset != self.maker_asset && amounts != 0
        let token_id = env::predecessor_account_id();

        match self.state {
            State::Init => {
                require!(token_id == self.maker_token_id, "wrong asset");
                // TODO: verify sender_id

                let refund = self
                    .maker_amount
                    .checked_sub(amount.0)
                    .ok_or("insufficient amount")
                    .unwrap_or_panic_static_str();

                self.state = State::Open;

                PromiseOrValue::Value(U128(refund))
            }
            State::Open => {
                // TODO: support adding more
                require!(token_id == self.taker_token_id, "wrong asset");
                let msg: TakerMessage = serde_json::from_str(&msg).unwrap();
                // TODO: check taker is in whitelist

                let refund = self
                    .taker_amount
                    .checked_sub(amount.0)
                    .ok_or("insufficient amount")
                    .unwrap_or_panic_str();

                self.settle(msg, refund)
            }
            _ => unimplemented!(),
        }
    }
}

impl Contract {
    fn settle(&mut self, taker_msg: TakerMessage, refund: u128) -> PromiseOrValue<U128> {
        self.state = State::Settling;
        // TODO: set state

        let finalize = Self::ext(env::current_account_id()).finalize(
            U128(self.taker_amount),
            self.receiver_msg.is_some(),
            U128(self.maker_amount),
            taker_msg.msg.is_some(),
            U128(refund),
        );

        Self::send(
            self.taker_token_id.clone(),
            self.taker_asset_receiver_id().clone(),
            self.taker_amount,
            // TODO: memo: what if it was withdrawal via PoA?
            // should we withdraw all at once or by parts?
            self.receiver_memo.clone(),
            self.receiver_msg.clone(),
        )
        .and(Self::send(
            self.maker_token_id.clone(),
            taker_msg.receiver_id,
            self.maker_amount,
            taker_msg.memo,
            taker_msg.msg,
        ))
        .then(finalize)
        .into()
    }

    fn send(
        token_contract_id: AccountId,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> Promise {
        // TODO: storage_deposits?

        let p = ext_ft_core::ext(token_contract_id)
            // TODO: static gas?
            .with_attached_deposit(NearToken::from_yoctonear(1));
        if let Some(msg) = msg {
            p.ft_transfer_call(receiver_id, U128(amount), memo, msg)
        } else {
            p.ft_transfer(receiver_id, U128(amount), memo)
        }
    }

    // TODO: docs
    /// Returns transferred amount
    fn ft_resolve_withdraw(result_idx: u64, amount: u128, is_call: bool) -> u128 {
        // TODO: check register length
        match env::promise_result(result_idx) {
            PromiseResult::Successful(value) => {
                if is_call {
                    // `ft_transfer_call` returns successfully transferred amount
                    serde_json::from_slice::<U128>(&value)
                        .unwrap_or_default()
                        .0
                        .min(amount)
                } else if value.is_empty() {
                    // `ft_transfer` returns empty result on success
                    amount
                } else {
                    0
                }
            }
            PromiseResult::Failed => {
                if is_call {
                    // do not refund on failed `ft_transfer_call` due to
                    // NEP-141 vulnerability: `ft_resolve_transfer` fails to
                    // read result of `ft_on_transfer` due to insufficient gas
                    amount
                } else {
                    0
                }
            }
        }
    }
}

#[near]
impl Contract {
    #[init]
    pub fn new(params: Params) -> Self {
        Self(params)
    }

    // pub fn cancel(&mut self) {
    //     require!(
    //         // TODO
    //         defuse_near_utils::BLOCK_TIMESTAMP.clone() > self.deadline,
    //         "deadline has not expired yet"
    //     );
    // }

    #[private]
    pub fn finalize(
        &mut self,
        taker_asset_amount: U128,
        taker_asset_is_call: bool,
        maker_asset_amount: U128,
        maker_asset_is_call: bool,
        refund: U128,
    ) -> U128 {
        // TODO: assert state is Settling?

        let [taker_asset_used, maker_asset_used] = [
            (0, taker_asset_amount, taker_asset_is_call),
            (1, maker_asset_amount, maker_asset_is_call),
        ]
        .map(|(result_idx, amount, is_call)| {
            Self::ft_resolve_withdraw(result_idx, amount.0, is_call)
        });

        if taker_asset_used == taker_asset_amount.0 && maker_asset_used == maker_asset_amount.0 {
            let _ = Promise::new(env::current_account_id())
                .delete_account(self.taker_asset_receiver_id().clone());
        } else {
            self.state = State::LostFound {
                taker_asset_lost: taker_asset_amount.0.saturating_sub(taker_asset_used),
                maker_asset_lost: maker_asset_amount.0.saturating_sub(maker_asset_amount.0),
            }
        }

        refund
    }

    // TODO
    pub fn lost_found(&mut self) -> Promise {
        let State::LostFound {
            // TODO: if we store a map for takers and their amounts,
            // then how can we allow anyone to transfer to them permissionlessly if the taer could have added `msg`?
            // do we want to store this data on a contract? what if we
            // run out of storage and the contract wouldn't be able to keep these refunds
            // But here is a point: MMs are smart and can use only
            // `ft_transfer()`s and manually index transfer events, right?
            taker_asset_lost,
            // what if maker asset was lost_found and he set msg param?
            maker_asset_lost,
        } = self.state
        else {
            env::panic_str("wrong state");
        };
        todo!()
    }
}
