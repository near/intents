use defuse::core::amounts::Amounts;
use defuse::core::intents::imt::{ImtBurn, ImtMint};
use defuse::core::token_id::TokenId;
use defuse::core::token_id::imt::ImtTokenId;
use defuse::core::token_id::nep141::Nep141TokenId;
use defuse_sandbox::assert_eq_defuse_event_logs;
use defuse_sandbox::extensions::defuse::event::ToEventLog;
use defuse_sandbox::extensions::defuse::intents::ExecuteIntentsExt;
use defuse_sandbox::extensions::defuse::signer::DefaultDefuseSignerExt;
use defuse_sandbox::extensions::mt::MtViewExt;
use rstest::rstest;

use crate::tests::defuse::env::Env;

#[rstest]
#[trace]
#[tokio::test]
async fn imt_burn_intent() {
    let env = Env::builder().build().await;

    let (user, other_user) = futures::join!(env.create_user(), env.create_user());

    let token_id = "sometoken.near".to_string();
    let memo = "Some memo";
    let amount = 1000;

    let mt_id = TokenId::from(ImtTokenId::new(user.id().clone(), token_id.to_string()));

    let mint_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [ImtMint {
                tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
                memo: Some(memo.to_string()),
                receiver_id: other_user.id().clone(),
                notification: None,
            }],
        )
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [mint_payload])
        .await
        .unwrap();

    let intent = ImtBurn {
        minter_id: user.id().clone(),
        tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
        memo: Some(memo.to_string()),
    };
    let burn_payload = other_user
        .sign_defuse_payload_default(&env.defuse, [intent.clone()])
        .await
        .unwrap();

    let result = env
        .simulate_and_execute_intents(env.defuse.id(), [burn_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(other_user.id(), &mt_id.to_string())
            .await
            .unwrap(),
        0
    );

    assert_eq_defuse_event_logs!(burn_payload.to_event_log(), result.logs());
}

#[rstest]
#[trace]
#[tokio::test]
async fn failed_to_burn_tokens_with_intent() {
    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let memo = "Some memo";
    let amount = 1000;

    // Only minted imt tokens can be burned
    let ft_id = TokenId::from(Nep141TokenId::new(ft));

    let withdraw_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [ImtBurn {
                minter_id: user.id().clone(),
                tokens: Amounts::new(vec![(ft_id.to_string(), amount)].into_iter().collect()),
                memo: Some(memo.to_string()),
            }],
        )
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [withdraw_payload])
        .await
        .unwrap_err();
}
