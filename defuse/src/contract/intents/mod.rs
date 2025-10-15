mod auth_call;
mod relayer;
mod state;

use defuse_core::{
    DefuseError,
    engine::{Engine, InspectorImpl, StateView},
    events::Dip4Event,
    payload::multi::MultiPayload,
};
use defuse_near_utils::UnwrapOrPanic;
use defuse_nep245::MtEvent;
use near_plugins::{Pausable, pause};
use near_sdk::{FunctionError, near};

use crate::intents::{Intents, SimulationOutput, StateOutput};

use super::{Contract, ContractExt};

#[near]
impl Intents for Contract {
    #[pause(name = "intents")]
    #[inline]
    fn execute_intents(&mut self, signed: Vec<MultiPayload>) {
        let mut inspector = InspectorImpl::new(self.event_sink_handle());

        Engine::new(&mut *self, &mut inspector)
            .execute_signed_intents(signed)
            .unwrap_or_panic()
            .as_mt_event()
            .as_ref()
            .map(MtEvent::emit);

        //NOTE: previously emitted in inspector drop
        if !inspector.intents_executed.is_empty() {
            self.emit_defuse_event(
                Dip4Event::IntentsExecuted(inspector.intents_executed.as_slice().into()).into(),
            );
        }
    }

    #[pause(name = "intents")]
    #[inline]
    fn simulate_intents(&self, signed: Vec<MultiPayload>) -> SimulationOutput {
        // NOTE: applies only to events routed through Contract::emit_defuse_event
        self.record_events_instead_of_emitting();

        let mut inspector = InspectorImpl::new(self.event_sink_handle());
        let engine = Engine::new(self.cached(), &mut inspector);
        let result = engine.execute_signed_intents(signed);

        let events = inspector.get_events();
        match result {
            // do not log transfers
            Ok(_) => SimulationOutput {
                intents_executed: inspector.intents_executed,
                events,
                min_deadline: inspector.min_deadline,
                invariant_violated: None,
                state: StateOutput {
                    fee: self.fee(),
                    current_salt: self.salts.current(),
                },
            },
            Err(DefuseError::InvariantViolated(v)) => SimulationOutput {
                intents_executed: inspector.intents_executed,
                events,
                min_deadline: inspector.min_deadline,
                invariant_violated: Some(v),
                state: StateOutput {
                    fee: self.fee(),
                    current_salt: self.salts.current(),
                },
            },
            Err(err) => err.panic(),
        }
    }
}
