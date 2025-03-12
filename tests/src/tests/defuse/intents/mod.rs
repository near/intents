use defuse::{
    core::{
        Deadline,
        intents::{DefuseIntents, tokens::Transfer},
        payload::multi::MultiPayload,
        tokens::{Amounts, TokenId},
    },
    intents::SimulationOutput,
};
use near_sdk::{AccountId, AccountIdRef};
use randomness::{Rng, make_true_rng};
use serde_json::json;

use crate::utils::mt::MtExt;

use super::{DefuseSigner, accounts::AccountManagerExt, env::Env};

mod ft_withdraw;
mod relayers;
mod token_diff;

pub trait ExecuteIntentsExt: AccountManagerExt {
    async fn defuse_execute_intents(
        &self,
        defuse_id: &AccountId,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<()>;
    async fn execute_intents(
        &self,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<()>;

    async fn defuse_simulate_intents(
        &self,
        defuse_id: &AccountId,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<SimulationOutput>;
    async fn simulate_intents(
        &self,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<SimulationOutput>;
}

impl ExecuteIntentsExt for near_workspaces::Account {
    async fn defuse_execute_intents(
        &self,
        defuse_id: &AccountId,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<()> {
        let args = json!({
            "signed": intents.into_iter().collect::<Vec<_>>(),
        });
        println!(
            "execute_intents({})",
            serde_json::to_string_pretty(&args).unwrap()
        );
        self.call(defuse_id, "execute_intents")
            .args_json(args)
            .max_gas()
            .transact()
            .await?
            .into_result()
            .map(|outcome| {
                println!(
                    "execute_intents: total_gas_burnt: {}, logs: {:#?}",
                    outcome.total_gas_burnt,
                    outcome.logs()
                );
            })
            .map_err(Into::into)
    }
    async fn execute_intents(
        &self,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<()> {
        self.defuse_execute_intents(self.id(), intents).await
    }

    async fn defuse_simulate_intents(
        &self,
        defuse_id: &AccountId,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<SimulationOutput> {
        let args = json!({
            "signed": intents.into_iter().collect::<Vec<_>>(),
        });
        println!(
            "simulate_intents({})",
            serde_json::to_string_pretty(&args).unwrap()
        );
        self.view(defuse_id, "simulate_intents")
            .args_json(args)
            .await?
            .json()
            .map_err(Into::into)
    }
    async fn simulate_intents(
        &self,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<SimulationOutput> {
        self.defuse_simulate_intents(self.id(), intents).await
    }
}

impl ExecuteIntentsExt for near_workspaces::Contract {
    async fn defuse_execute_intents(
        &self,
        defuse_id: &AccountId,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<()> {
        self.as_account()
            .defuse_execute_intents(defuse_id, intents)
            .await
    }
    async fn execute_intents(
        &self,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<()> {
        self.as_account().execute_intents(intents).await
    }

    async fn defuse_simulate_intents(
        &self,
        defuse_id: &AccountId,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<SimulationOutput> {
        self.as_account()
            .defuse_simulate_intents(defuse_id, intents)
            .await
    }
    async fn simulate_intents(
        &self,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<SimulationOutput> {
        self.as_account().simulate_intents(intents).await
    }
}

#[tokio::test]
async fn test_simulate_is_view_method() {
    let env = Env::new().await;

    let ft1 = TokenId::Nep141(env.ft1.clone());

    // deposit
    env.defuse_ft_mint(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();

    env.defuse
        .simulate_intents([env.user1.sign_defuse_message(
            env.defuse.id(),
            make_true_rng().random(),
            Deadline::MAX,
            DefuseIntents {
                intents: [Transfer {
                    receiver_id: env.user2.id().clone(),
                    tokens: Amounts::new([(ft1.clone(), 1000)].into_iter().collect()),
                    memo: None,
                }
                .into()]
                .into(),
            },
        )])
        .await
        .unwrap()
        .into_result()
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(env.user1.id(), &ft1.to_string())
            .await
            .unwrap(),
        1000
    );
    assert_eq!(
        env.defuse
            .mt_balance_of(env.user2.id(), &ft1.to_string())
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn test_webauthn() {
    const SIGNER_ID: &AccountIdRef =
        AccountIdRef::new_or_panic("0x3602b546589a8fcafdce7fad64a46f91db0e4d50");

    let env = Env::new().await;

    let ft1 = TokenId::Nep141(env.ft1.clone());

    // deposit
    env.defuse_ft_mint(&env.ft1, 2000, &SIGNER_ID.to_owned())
        .await
        .unwrap();

    env.defuse
        .execute_intents([serde_json::from_str(r#"{
  "standard": "webauthn",
  "payload": "{\"signer_id\":\"0x3602b546589a8fcafdce7fad64a46f91db0e4d50\",\"verifying_contract\":\"defuse.test.near\",\"deadline\":\"2050-03-30T00:00:00Z\",\"nonce\":\"A3nsY1GMVjzyXL3mUzOOP3KT+5a0Ruy+QDNWPhchnxM=\",\"intents\":[{\"intent\":\"transfer\",\"receiver_id\":\"user1.test.near\",\"tokens\":{\"nep141:ft1.poa-factory.test.near\":\"1000\"}}]}",
  "public_key": "p256:2V8Np9vGqLiwVZ8qmMmpkxU7CTRqje4WtwFeLimSwuuyF1rddQK5fELiMgxUnYbVjbZHCNnGc6fAe4JeDcVxgj3Q",
  "signature": "p256:2wpTbs61923xQU9L4mqBGSdHSdv5mqMn3zRA2tFmDirm8t4mx1PYAL7Vhe9uta4WMbHoMMTBZ8KQSM7nWug3Nrc7",
  "client_data_json": "{\"type\":\"webauthn.get\",\"challenge\":\"DjS-6fxaPS3avW-4ls8dDYAynCmsAXWCF86cJBTkHbs\",\"origin\":\"https://defuse-widget-git-feat-passkeys-defuse-94bbc1b2.vercel.app\"}",
  "authenticator_data": "933cQogpBzE3RSAYSAkfWoNEcBd3X84PxE8iRrRVxMgdAAAAAA=="
}"#).unwrap(), serde_json::from_str(r#"{
  "standard": "webauthn",
  "payload": "{\"signer_id\":\"0x3602b546589a8fcafdce7fad64a46f91db0e4d50\",\"verifying_contract\":\"defuse.test.near\",\"deadline\":\"2050-03-30T00:00:00Z\",\"nonce\":\"B3nsY1GMVjzyXL3mUzOOP3KT+5a0Ruy+QDNWPhchnxM=\",\"intents\":[{\"intent\":\"transfer\",\"receiver_id\":\"user1.test.near\",\"tokens\":{\"nep141:ft1.poa-factory.test.near\":\"1000\"}}]}",
  "public_key": "p256:2V8Np9vGqLiwVZ8qmMmpkxU7CTRqje4WtwFeLimSwuuyF1rddQK5fELiMgxUnYbVjbZHCNnGc6fAe4JeDcVxgj3Q",
  "signature": "p256:5Zq1w2ntVi5EowuKPnaSyuM2XB3JsQZub5CXB1fHsP6MWMSV1RXEoqpgVn5kNK43ZiUoXGBKVvUSS3DszwWCWgG6",
  "client_data_json": "{\"type\":\"webauthn.get\",\"challenge\":\"6ULo-LNIjd8Gh1mdxzUdHzv2AuGDWMchOORdDnaLXHc\",\"origin\":\"https://defuse-widget-git-feat-passkeys-defuse-94bbc1b2.vercel.app\"}",
  "authenticator_data": "933cQogpBzE3RSAYSAkfWoNEcBd3X84PxE8iRrRVxMgdAAAAAA=="
}"#).unwrap()])
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(env.user1.id(), &ft1.to_string())
            .await
            .unwrap(),
        2000
    );
    assert_eq!(
        env.defuse
            .mt_balance_of(&SIGNER_ID.to_owned(), &ft1.to_string())
            .await
            .unwrap(),
        0
    );
}
