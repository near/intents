mod execute;
mod relayer;
mod simulate;
mod state;
mod tokens;

use defuse_core::{
    crypto::{Payload, SignedPayload},
    engine::{Engine, State, StateView},
    intents::{DefuseIntents, ExecutableIntent},
    payload::{multi::MultiPayload, DefusePayload, ExtractDefusePayload},
    DefuseError, Result,
};
use defuse_near_utils::{UnwrapOrPanic, CURRENT_ACCOUNT_ID};
use execute::ExecuteInspector;
use near_plugins::{pause, Pausable};
use near_sdk::near;
use simulate::SimulateInspector;

use crate::intents::{Intents, SimulationOutput, StateOutput, TokenDiffOutput};

use super::{Contract, ContractExt};

#[near]
impl Intents for Contract {
    #[pause(name = "intents")]
    #[inline]
    fn execute_intents(&mut self, intents: Vec<MultiPayload>) {
        let mut insp = ExecuteInspector::default();
        let mut engine = Engine::new(self, &mut insp);
        engine
            .execute_signed_intents::<_, DefuseIntents>(intents)
            .unwrap_or_panic();
        if let Some(event) = engine.finalize().unwrap_or_panic().into_event() {
            event.emit();
        }

        // TODO: finalize transfers & emit?
        // TODO: finalize?
    }

    #[pause(name = "intents")]
    #[inline]
    fn simulate_intents(&self, intents: Vec<MultiPayload>) -> SimulationOutput {
        let mut inspector = SimulateInspector::default();
        let mut engine = Engine::new(self.cached(), &mut inspector);
        engine
            .execute_signed_intents::<_, DefuseIntents>(intents)
            .unwrap_or_panic();
        SimulationOutput {
            intents_executed: inspector.intents_executed,
            min_deadline: inspector.min_deadline,
            token_diff: TokenDiffOutput { closure: todo!() },
            state: StateOutput { fee: self.fee() },
        }
    }
}
