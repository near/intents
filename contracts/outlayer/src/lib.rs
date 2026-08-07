use std::collections::BTreeMap;
mod state;

pub use self::state::*;

pub trait Outlayer {
    // TODO: allow to mutate itself?...
    // TODO: must be payable
    // TODO: add pre-fetch storage slots
    fn execute(&mut self, input: String) -> ExecutionOutcome;
}

pub struct ExecutionOutcome {
    // TODO: id?...
    // TODO: how to identify currently run code?...
    pub code_hash: [u8; 32],
    pub output: String,
    pub logs: String,
    pub state: BTreeMap<Vec<u8>, StateEntry>,
    // TODO: refund:...
}
