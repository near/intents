use chrono::{DateTime, Utc};
use defuse_borsh_utils::adapters::{As, TimestampNanoSeconds};
use defuse_near_utils::UnwrapOrPanicError;
use near_contract_standards::fungible_token::{core::ext_ft_core, receiver::FungibleTokenReceiver};
use near_sdk::{
    AccountId, NearToken, PanicOnDefault, Promise, PromiseOrValue, PromiseResult, env,
    json_types::U128, near, require, serde_json,
};

#[near(contract_state, serializers = [borsh, json])]
#[derive(Debug, PanicOnDefault)]
pub struct Contract {
    pub maker_token_id: AccountId,
    pub maker_amount: u128,

    pub taker_token_id: AccountId,
    pub taker_amount: u128,
    pub taker_asset_receiver_id: AccountId,
    // TODO: taker_asset_ft_msg only if nep141
    pub state: State,

    // #[borsh(
    //     serialize_with = "As::<TimestampNanoSeconds>::serialize",
    //     deserialize_with = "As::<TimestampNanoSeconds>::deserialize"
    // )]
    // pub deadline: DateTime<Utc>,
    pub salt: [u8; 4],
    // TODO: fees:
    // * hard-code protocol fee_collector
    // * app fees
}

pub enum EscrowAsset {
    Nep141 {
        contract_id: AccountId,
        receiver_id: AccountId,
        msg: Option<String>,
    }
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
        taker_token_ok: bool,
        maker_token_ok: bool,
    },
}

#[near(serializers = [json])]
pub struct TakerMessage {
    pub receiver_id: AccountId,
}

#[near]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
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

        ext_ft_core::ext(self.taker_token_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            // .with_static_gas(static_gas)
            .ft_transfer(
                self.taker_asset_receiver_id.clone(),
                U128(self.taker_amount),
                None,
            )
            .and(
                ext_ft_core::ext(self.maker_token_id.clone())
                    .with_attached_deposit(NearToken::from_yoctonear(1))
                    .ft_transfer(taker_msg.receiver_id, U128(self.maker_amount), None),
            )
            .then(Self::ext(env::current_account_id()).finalize(U128(refund)))
            .into()
    }
}

#[near]
impl Contract {
    #[init]
    pub fn new(config: Contract) -> Self {
        config
    }

    #[private]
    pub fn finalize(&mut self, refund: U128) -> U128 {
        let [taker_token_ok, maker_token_ok] = [0, 1].map(
            |n| matches!(env::promise_result(n), PromiseResult::Successful(v) if v.is_empty()),
        );

        if taker_token_ok && maker_token_ok {
            let _ = Promise::new(env::current_account_id())
                .delete_account(self.taker_asset_receiver_id.clone());
        } else {
            self.state = State::LostFound {
                taker_token_ok,
                maker_token_ok,
            }
        }

        refund
    }

    // TODO
    // pub fn lost_found(&mut self) -> Promise {
    //     let State::LostFound {
    //         taker_token_ok,
    //         maker_token_ok,
    //     } = self.state
    //     else {
    //         env::panic_str("wrong state");
    //     };
    // }
}
