use arbitrary::{Arbitrary, Unstructured};
use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;
use defuse::core::ton_connect::tlb_ton::MsgAddress;
use defuse::core::{
    Deadline, Nonce,
    accounts::{AccountEvent, NonceEvent},
    amounts::Amounts,
    crypto::Payload,
    events::DefuseEvent,
    intents::{
        DefuseIntents, IntentEvent,
        tokens::{FtWithdraw, Transfer},
    },
    payload::{DefusePayload, ExtractDefusePayload},
};
use defuse::extensions::intents::ExecuteIntentsExt;
use defuse::extensions::signer::Signer;
use defuse_randomness::Rng;
use defuse_sandbox::extensions::mt::MtViewExt;
use defuse_test_utils::random::rng;
use near_sdk::{AccountId, AccountIdRef, AsNep297Event, CryptoHash};
use rstest::rstest;
use std::borrow::Cow;

use super::{DefuseSigner, env::Env};
use crate::tests::defuse::SigningStandard;

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

    pub fn into_event(self) -> DefuseEvent<'static> {
        DefuseEvent::IntentsExecuted(
            vec![IntentEvent::new(
                AccountEvent::new(self.0, NonceEvent::new(self.1)),
                self.2,
            )]
            .into(),
        )
    }
}

mod ft_withdraw;
mod legacy_nonce;
mod native_withdraw;
mod public_key;
mod relayers;
mod simulate;
mod token_diff;
mod transfer;

pub const DUMMY_MSG_ADDRESS: MsgAddress = MsgAddress {
    workchain_id: 1234i32,
    address: [12u8; 32],
};

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_is_view_method(#[notrace] mut rng: impl Rng) {
    use defuse::core::accounts::TransferEvent;

    let env = Env::builder().build().await;

    let (user, other_user, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    let ft_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    env.initial_ft_storage_deposit(vec![user.id(), other_user.id()], vec![ft.id()])
        .await;

    // deposit
    env.defuse_ft_deposit_to(ft.id(), 1000, user.id(), None)
        .await
        .unwrap();

    let nonce = rng.random();

    let transfer_intent = Transfer {
        receiver_id: other_user.id().clone(),
        tokens: Amounts::new(std::iter::once((ft_id.clone(), 1000)).collect()),
        memo: None,
        notification: None,
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
        .simulate_intents(env.defuse.id(), [transfer_intent_payload.clone()])
        .await
        .unwrap();

    assert_eq!(result.report.intents_executed.len(), 1);

    // Prepare expected transfer event
    let expected_log = DefuseEvent::Transfer(Cow::Owned(vec![IntentEvent {
        intent_hash: transfer_intent_payload.hash(),
        event: AccountEvent {
            account_id: user.id().clone().into(),
            event: TransferEvent {
                receiver_id: Cow::Borrowed(&transfer_intent.receiver_id),
                tokens: transfer_intent.tokens,
                memo: Cow::Borrowed(&transfer_intent.memo),
            },
        },
    }]))
    .to_nep297_event()
    .to_event_log();

    assert!(result.report.logs.iter().any(|log| log == &expected_log));
    assert_eq!(
        result
            .report
            .intents_executed
            .first()
            .unwrap()
            .event
            .event
            .nonce,
        nonce
    );
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
async fn webauthn() {
    const SIGNER_ID: &AccountIdRef =
        AccountIdRef::new_or_panic("0x3602b546589a8fcafdce7fad64a46f91db0e4d50");

    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(
        env.create_named_user("user1"),
        env.create_named_token("ft1")
    );

    let ft_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    // deposit
    env.defuse_ft_deposit_to(ft.id(), 2000, SIGNER_ID, None)
        .await
        .unwrap();

    env
        .simulate_and_execute_intents(env.defuse.id(), [serde_json::from_str(r#"{
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
async fn ton_connect_sign_intent_example() {
    let env: Env = Env::builder().build().await;

    let ft_id: AccountId = "ft.test.near".parse().unwrap();

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
    let nonce = env.get_unique_nonce(None).await.unwrap();

    let payload = defuse::core::ton_connect::TonConnectPayload {
        address: DUMMY_MSG_ADDRESS,
        domain: "example.com".to_string(),
        timestamp: defuse_near_utils::time::now(),
        payload: defuse::core::ton_connect::TonConnectPayloadSchema::text(
            serde_json::to_string(&DefusePayload {
                signer_id: "alice.near".parse().unwrap(),
                verifying_contract: "intent.near".parse().unwrap(),
                deadline: Deadline::timeout(std::time::Duration::from_secs(120)),
                nonce,
                message: intents,
            })
            .unwrap(),
        ),
    };

    let account = env.sandbox().create_account("alice.near").await.unwrap();
    let signed = account.sign_ton_connect(payload);

    let _decoded_payload: DefusePayload<DefuseIntents> = signed.extract_defuse_payload().unwrap();
}
