use defuse_core::payload::multi::MultiPayload;
use defuse_sandbox::{
    Account, SigningAccount, anyhow, api::types::transaction::result::ExecutionSuccess,
    tx::FnCallBuilder,
};
use near_sdk::{
    AccountIdRef,
    serde_json::{self, json},
};

use crate::intents::SimulationOutput;

#[allow(async_fn_in_trait)]
pub trait ExecuteIntentsExt {
    async fn execute_intents(
        &self,
        defuse_id: &AccountIdRef,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<ExecutionSuccess>;

    async fn simulate_intents(
        &self,
        defuse_id: &AccountIdRef,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<SimulationOutput>;

    async fn simulate_and_execute_intents(
        &self,
        defuse_id: &AccountIdRef,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<ExecutionSuccess> {
        let intents = intents.into_iter().collect::<Vec<_>>();
        let simulation_result = self.simulate_intents(defuse_id, intents.clone()).await;

        self.execute_intents(defuse_id, intents)
            .await
            // return simulation_err if execute_ok
            .and_then(|res| simulation_result.map(|_| res))
    }
}

impl ExecuteIntentsExt for SigningAccount {
    async fn execute_intents(
        &self,
        defuse_id: &AccountIdRef,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<ExecutionSuccess> {
        let args = json!({
            "signed": intents.into_iter().collect::<Vec<_>>(),
        });

        println!(
            "execute_intents({})",
            serde_json::to_string_pretty(&args).unwrap()
        );

        self.tx(defuse_id.into())
            .function_call(FnCallBuilder::new("execute_intents").json_args(&args))
            .await
            .inspect(|outcome| {
                println!("execute_intents: {outcome:#?}");
            })
    }

    async fn simulate_intents(
        &self,
        defuse_id: &AccountIdRef,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<SimulationOutput> {
        let args = json!({
            "signed": intents.into_iter().collect::<Vec<_>>(),
        });

        println!(
            "simulate_intents({})",
            serde_json::to_string_pretty(&args).unwrap()
        );

        let defuse = Account::new(defuse_id.into(), self.network_config().clone());

        defuse
            .call_view_function_json("simulate_intents", args)
            .await
    }
}
