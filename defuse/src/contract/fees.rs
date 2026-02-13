use core::mem;
use std::borrow::Cow;

use defuse_core::{
    events::DefuseIntentEmit,
    fees::{FeeChangedEvent, FeeCollectorChangedEvent, Pips},
};
use near_plugins::{AccessControllable, Pausable, access_control_any, pause};
use near_sdk::{AccountId, assert_one_yocto, near, require};

use crate::fees::FeesManager;

use super::{Contract, ContractExt, Role};

#[near]
impl FeesManager for Contract {
    #[pause(name = "intents")]
    #[access_control_any(roles(Role::DAO, Role::FeesManager))]
    #[payable]
    fn set_fee(&mut self, #[allow(unused_mut)] mut fee: Pips) {
        assert_one_yocto();
        require!(self.fees.fee != fee, "same");
        mem::swap(&mut self.fees.fee, &mut fee);
        FeeChangedEvent {
            old_fee: fee,
            new_fee: self.fees.fee,
        }
        .emit();
    }

    fn fee(&self) -> Pips {
        self.fees.fee
    }

    #[pause(name = "intents")]
    #[access_control_any(roles(Role::DAO, Role::FeesManager))]
    #[payable]
    fn set_fee_collector(&mut self, #[allow(unused_mut)] mut fee_collector: AccountId) {
        assert_one_yocto();
        require!(self.fees.fee_collector != fee_collector, "same");
        mem::swap(&mut self.fees.fee_collector, &mut fee_collector);
        FeeCollectorChangedEvent {
            old_fee_collector: fee_collector.into(),
            new_fee_collector: Cow::Borrowed(self.fees.fee_collector.as_ref()),
        }
        .emit();
    }

    fn fee_collector(&self) -> &AccountId {
        &self.fees.fee_collector
    }
}
