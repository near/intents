use crate::{tests::defuse::DefuseSigner, tests::defuse::accounts::AccountManagerExt, tests::defuse::env::Env};
use crate::tests::defuse::intents::ExecuteIntentsExt;
use crate::tests::defuse::SigningStandard;
use crate::tests::utils::AsNearSdkLog;
use crate::utils::{crypto::Signer, mt::MtExt, test_log::TestLog};
use defuse_crypto::Payload;
use arbitrary::{Arbitrary, Unstructured};
use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;
use defuse::{
    core::{
        Deadline,
        accounts::AccountEvent,
        amounts::Amounts,
        events::DefuseEvent,
        intents::{
            DefuseIntents, IntentEvent,
            tokens::{FtWithdraw, Transfer},
        },
        payload::{DefusePayload, ExtractDefusePayload, multi::MultiPayload},
    },
    intents::SimulationOutput,
};
use defuse_randomness::Rng;
use defuse_test_utils::random::rng;
use near_sdk::{AccountId, AccountIdRef};
use rstest::rstest;
use serde_json::json;
use std::borrow::Cow;


#[tokio::test]
#[rstest]
#[trace]
async fn simulate_transfer_intent(
    #[notrace] mut rng: impl Rng,
) {
    let env = Env::builder()
        .no_registration(true)
        .build()
        .await;

    let ft1 = TokenId::from(Nep141TokenId::new(env.ft1.clone()));

    // deposit
    env.defuse_ft_deposit_to(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();

    let nonce = rng.random();

    let transfer_intent = Transfer {
        receiver_id: env.user2.id().clone(),
        tokens: Amounts::new(std::iter::once((ft1.clone(), 1000)).collect()),
        memo: None,
    };
    let transfer_intent_payload = env.user1.sign_defuse_message(
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

    assert_eq!(result.intents_executed.len(), 1);

    // Prepare expected transfer event
    let expected_log = DefuseEvent::Transfer(Cow::Owned(vec![IntentEvent {
            intent_hash: transfer_intent_payload.hash(),
            event: AccountEvent {
                account_id: env.user1.id().clone().into(),
                event: Cow::Owned(transfer_intent),
            },
        }]))
        .as_near_sdk_log();

    result.logs.iter().for_each(|log| println!("{}", log));
    println!("{}", near_sdk::serde_json::to_string_pretty(&result).unwrap());

    assert!(result.logs.iter().any(|log| log == &expected_log));
    //TODO: update
    // assert_eq!(
    //     result.intents_executed.first().unwrap().event.event.nonce,
    //     nonce
    // );
    result.into_result().unwrap();

}


