use super::{DefuseSigner, accounts::AccountManagerExt, env::Env};
use crate::tests::defuse::SigningStandard;
use crate::utils::{crypto::Signer, mt::MtExt, test_log::TestLog};
use arbitrary::{Arbitrary, Unstructured};
use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;
use defuse::{
    core::{
        Deadline, Nonce,
        accounts::{AccountEvent, NonceEvent},
        amounts::Amounts,
        crypto::Payload,
        events::DefuseEvent,
        intents::{
            DefuseIntents, IntentEvent,
            tokens::{FtWithdraw, Transfer},
        },
        payload::{DefusePayload, ExtractDefusePayload, multi::MultiPayload},
    },
    intents::SimulationOutput,
};
use defuse_near_utils::NearSdkLog;
use defuse_randomness::Rng;
use defuse_test_utils::random::rng;
use near_sdk::{AccountId, AccountIdRef, CryptoHash};
use rstest::rstest;
use serde_json::json;
use std::borrow::Cow;

pub struct AccountNonceIntentEvent(AccountId, Nonce, CryptoHash);

impl AccountNonceIntentEvent {
    pub fn new(
        account_id: impl AsRef<AccountIdRef> + Clone,
        nonce: Nonce,
        payload: &impl Payload,
    ) -> Self {
        let acc = account_id.as_ref().to_owned();
        Self(acc, nonce, payload.hash())
    }

    pub fn into_event_log(self) -> String {
        DefuseEvent::IntentsExecuted(
            vec![IntentEvent::new(
                AccountEvent::new(self.0, NonceEvent::new(self.1)),
                self.2,
            )]
            .into(),
        )
        .to_near_sdk_log()
    }
}

mod ft_withdraw;
mod native_withdraw;
mod public_key;
mod relayers;
mod simulate;
mod token_diff;

pub trait ExecuteIntentsExt: AccountManagerExt {
    async fn defuse_execute_intents(
        &self,
        defuse_id: &AccountId,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<TestLog>;

    async fn execute_intents(
        &self,
        defuse_id: &AccountId,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<TestLog> {
        let intents = intents.into_iter().collect::<Vec<_>>();
        let simulation_result = self
            .defuse_simulate_intents(defuse_id, intents.clone())
            .await;

        self.defuse_execute_intents(defuse_id, intents)
            .await
            // return simulation_err if execute_ok
            .and_then(|res| simulation_result.map(|_| res))
    }

    async fn execute_intents_without_simulation(
        &self,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<TestLog>;

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
    ) -> anyhow::Result<TestLog> {
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
            .inspect(|outcome| {
                println!("execute_intents: {outcome:#?}");
            })
            .map(Into::into)
            .map_err(Into::into)
    }

    async fn execute_intents_without_simulation(
        &self,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<TestLog> {
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
    ) -> anyhow::Result<TestLog> {
        self.as_account()
            .defuse_execute_intents(defuse_id, intents)
            .await
    }
    async fn execute_intents(
        &self,
        defuse_id: &AccountId,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<TestLog> {
        self.as_account().execute_intents(defuse_id, intents).await
    }

    async fn execute_intents_without_simulation(
        &self,
        intents: impl IntoIterator<Item = MultiPayload>,
    ) -> anyhow::Result<TestLog> {
        self.as_account()
            .execute_intents_without_simulation(intents)
            .await
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
#[rstest]
#[trace]
async fn simulate_is_view_method(
    #[notrace] mut rng: impl Rng,
    #[values(false, true)] no_registration: bool,
) {
    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let (user, other_user, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    env.initial_ft_storage_deposit(vec![user.id(), other_user.id()], vec![&ft])
        .await;

    // deposit
    env.defuse_ft_deposit_to(&ft, 1000, user.id())
        .await
        .unwrap();

    let nonce = rng.random();

    let transfer_intent = Transfer {
        receiver_id: other_user.id().clone(),
        tokens: Amounts::new(std::iter::once((ft_id.clone(), 1000)).collect()),
        memo: None,
    };
    let transfer_intent_payload = user.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![transfer_intent.clone().into()],
        },
    );
    let result = env
        .defuse
        .simulate_intents([transfer_intent_payload.clone()])
        .await
        .unwrap();

    assert_eq!(result.report.intents_executed.len(), 1);

    // Prepare expected transfer event
    let expected_log = DefuseEvent::Transfer(Cow::Owned(vec![IntentEvent {
        intent_hash: transfer_intent_payload.hash(),
        event: AccountEvent {
            account_id: user.id().clone().into(),
            event: Cow::Owned(transfer_intent),
        },
    }]))
    .to_near_sdk_log();

    assert!(result.report.logs.iter().any(|log| log == &expected_log));
    //TODO: update
    // assert_eq!(
    //     result.report.intents_executed.first().unwrap().event.event.nonce,
    //     nonce
    // );
    result.into_result().unwrap();

    // Verify balances haven't changed (simulate is a view method)
    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        1000
    );
    assert_eq!(
        env.defuse
            .mt_balance_of(other_user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
#[rstest]
async fn webauthn(#[values(false, true)] no_registration: bool) {
    const SIGNER_ID: &AccountIdRef =
        AccountIdRef::new_or_panic("0x3602b546589a8fcafdce7fad64a46f91db0e4d50");

    let env = Env::builder()
        .no_registration(no_registration)
        .build()
        .await;

    let (user, ft) = futures::join!(
        env.create_named_user("user1"),
        env.create_named_token("ft1")
    );

    let ft_id = TokenId::from(Nep141TokenId::new(ft.clone()));

    env.initial_ft_storage_deposit(vec![user.id()], vec![&ft])
        .await;

    // deposit
    env.defuse_ft_deposit_to(&ft, 2000, &SIGNER_ID.to_owned())
        .await
        .unwrap();

    env.defuse
        .execute_intents(env.defuse.id(), [serde_json::from_str(r#"{
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
            .mt_balance_of(user.id(), &ft_id.to_string())
            .await
            .unwrap(),
        2000
    );
    assert_eq!(
        env.defuse
            .mt_balance_of(&SIGNER_ID.to_owned(), &ft_id.to_string())
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn ton_connect_sign_intent_example(#[notrace] mut rng: impl Rng) {
    use defuse::core::ton_connect::tlb_ton::MsgAddress;

    let env: Env = Env::builder().no_registration(false).build().await;

    let ft_id: AccountId = "ft.test.near".parse().unwrap();
    let address = MsgAddress::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 32]>())).unwrap();

    let intents = DefuseIntents {
        intents: [FtWithdraw {
            token: ft_id,
            receiver_id: "bob.near".parse().unwrap(),
            amount: 1000.into(),
            memo: None,
            msg: None,
            storage_deposit: None,
            min_gas: None,
        }
        .into()]
        .into(),
    };

    let payload = defuse::core::ton_connect::TonConnectPayload {
        address,
        domain: "example.com".to_string(),
        timestamp: defuse_near_utils::time::now(),
        payload: defuse::core::ton_connect::TonConnectPayloadSchema::Text {
            text: serde_json::to_string(&DefusePayload {
                signer_id: "alice.near".parse().unwrap(),
                verifying_contract: "intent.near".parse().unwrap(),
                deadline: Deadline::timeout(std::time::Duration::from_secs(120)),
                nonce: rng.random(),
                message: intents,
            })
            .unwrap(),
        },
    };

    let root_secret_key = env.sandbox().root_account().secret_key();

    let worker = env.sandbox().worker().clone();

    let account = near_workspaces::Account::from_secret_key(
        "alice.near".parse().unwrap(),
        root_secret_key.clone(),
        &worker,
    );

    let signed = account.sign_ton_connect(payload);

    let _decoded_payload: DefusePayload<DefuseIntents> = signed.extract_defuse_payload().unwrap();
}
