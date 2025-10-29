mod tokens;

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use defuse_token_id::TokenId;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{json_types::U128, near, serde_json, AccountId, PromiseOrValue};

use crate::Escrow;


pub enum State {
    Init,
    ReceivedAsset1,
    ReceivedAsset2,
    // TODO: intermediary states
    // TODO: queue via yield/resume
}


pub struct EscrowState {
    // TODO: whitelist maker?
    pub assets: [TokenId; 2], // TODO: assert != maker_asset
    pub amounts: [u128; 2],

    pub deadline: DateTime<Utc>,

    // deadline:
    pub taker_whitelist: HashSet<AccountId>,

    pub state: State,
    // TODO: fees
}

impl FungibleTokenReceiver for EscrowState {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        match self.state {
            State::Init => {
                
            },
        }
    }
}
