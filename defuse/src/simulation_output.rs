use defuse_core::{
    Deadline, Result, Salt,
    accounts::{AccountEvent, NonceEvent},
    engine::deltas::InvariantViolated,
    fees::Pips,
    intents::IntentEvent,
};

// #[cfg_attr(
//     all(feature = "abi", not(target_arch = "wasm32")),
//     serde_as(schemars = true)
// )]
use near_sdk::near;
// use serde_with::serde_as;

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SimulationReport {
    pub intents_executed: Vec<IntentEvent<AccountEvent<'static, NonceEvent>>>,
    pub logs: Vec<String>,
    pub min_deadline: Deadline,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SimulationOutput {
    #[serde(flatten)]
    pub report: SimulationReport,

    /// Unmatched token deltas needed to keep the invariant.
    /// If not empty, can be used along with fee to calculate `token_diff` closure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invariant_violated: Option<InvariantViolated>,

    /// Additional info about current state
    pub state: StateOutput,
}

impl SimulationOutput {
    pub fn into_result(self) -> Result<(), InvariantViolated> {
        if let Some(unmatched_deltas) = self.invariant_violated {
            return Err(unmatched_deltas);
        }
        Ok(())
    }
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct StateOutput {
    pub fee: Pips,

    pub current_salt: Salt,
}
