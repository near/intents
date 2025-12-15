use defuse_core::payload::multi::MultiPayload;
use defuse_sandbox::{
    Account, SigningAccount, anyhow, api::types::transaction::result::ExecutionSuccess,
    tx::FnCallBuilder,
};
use near_sdk::{
    AccountId,
    serde_json::{self, json},
};

use crate::intents::SimulationOutput;

pub trait ExecuteIntentsExt {
    async fn execute_intents(
        &self,
        contract_id: impl Into<AccountId>,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<ExecutionSuccess>;

    async fn simulate_and_execute_intents(
        &self,
        contract_id: impl Into<AccountId>,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<ExecutionSuccess>;
}

impl ExecuteIntentsExt for SigningAccount {
    async fn execute_intents(
        &self,
        contract_id: impl Into<AccountId>,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<ExecutionSuccess> {
        let args = json!({
            "signed": intents.into_iter().collect::<Vec<_>>(),
        });

        println!(
            "execute_intents({})",
            serde_json::to_string_pretty(&args).unwrap()
        );

        self.tx(contract_id)
            .function_call(FnCallBuilder::new("execute_intents").json_args(&args))
            .await
    }

    async fn simulate_and_execute_intents(
        &self,
        contract_id: impl Into<AccountId>,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<ExecutionSuccess> {
        let contract_id = contract_id.into();
        let intents = intents.into_iter().collect::<Vec<_>>();

        let simulation_result = Account::new(contract_id.clone(), self.network_config().clone())
            .simulate_intents(intents.clone())
            .await;

        self.execute_intents(contract_id, intents)
            .await
            .and_then(|res| simulation_result.map(|_| res))
    }
}

pub trait SimulateIntents {
    async fn simulate_intents(
        &self,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<SimulationOutput>;
}

impl SimulateIntents for Account {
    async fn simulate_intents(
        &self,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<SimulationOutput> {
        let args = json!({
            "signed": intents.into_iter().collect::<Vec<_>>(),
        });

        println!(
            "simulate_intents({})",
            serde_json::to_string_pretty(&args).unwrap()
        );

        self.call_view_function_json("simulate_intents", args).await
    }
}
