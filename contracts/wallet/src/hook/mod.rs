pub mod keepalive;

use core::fmt::{self, Display};

use near_sdk::near;

use crate::{Actor, Request, Wallet};

// TODO: docs
// TODO add to contract_metadata
pub trait WalletHook: Wallet {
    fn hook(&self) -> String;
}

pub trait Hook {
    // TODO: give (read?) access to the rest of the state?
    // TODO: custom ops for state mutations?
    fn on_request(&mut self, request: &Request, actor: Actor<'_>);
}

#[near(serializers = [borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Noop;

impl Display for Noop {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl Hook for Noop {
    fn on_request(&mut self, _request: &Request, _actor: Actor<'_>) {}
}
