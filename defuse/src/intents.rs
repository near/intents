use defuse_core::payload::multi::MultiPayload;

use near_plugins::AccessControllable;
use near_sdk::{Promise, PublicKey, ext_contract};

use crate::{fees::FeesManager, salts::SaltManager};

pub use crate::simulation_output::{SimulationOutput, StateOutput};

#[ext_contract(ext_intents)]
pub trait Intents: FeesManager + SaltManager {
    fn execute_intents(&mut self, signed: Vec<MultiPayload>);

    fn simulate_intents(&self, signed: Vec<MultiPayload>) -> SimulationOutput;
}

#[ext_contract(ext_relayer_keys)]
pub trait RelayerKeys: AccessControllable {
    /// Adds access key for calling `execute_signed_intents`
    /// with allowance passed as attached deposit via `#[payable]`
    /// NOTE: requires 1yN for security purposes
    fn add_relayer_key(&mut self, public_key: PublicKey) -> Promise;

    fn do_add_relayer_key(&mut self, public_key: PublicKey);

    /// NOTE: requires 1yN for security purposes
    fn delete_relayer_key(&mut self, public_key: PublicKey) -> Promise;
}
