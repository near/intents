use std::borrow::Cow;

use defuse::core::accounts::AccountEvent;
use defuse::core::amounts::Amounts;
use defuse::core::crypto::Payload;
use defuse::core::events::DefuseEvent;
use defuse::core::intents::IntentEvent;
use defuse::core::intents::tokens::{ImtBurn, ImtMint};
use defuse::core::token_id::TokenId;
use defuse::nep245::{MtBurnEvent, MtEvent};
use defuse::sandbox_ext::intents::ExecuteIntentsExt;
use defuse_escrow_swap::token_id::imt::ImtTokenId;
use defuse_sandbox::assert_a_contains_b;
use defuse_sandbox::extensions::mt::MtViewExt;
use near_sdk::json_types::U128;
use rstest::rstest;

use near_sdk::AsNep297Event;

use crate::tests::defuse::DefuseSignerExt;
use crate::tests::defuse::env::Env;

#[rstest]
#[trace]
#[tokio::test]
async fn mt_burn_intent() {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let token_id = "sometoken.near".to_string();
    let memo = "Some memo";
    let amount = 1000;

    let mint_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [ImtMint {
                tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
                memo: Some(memo.to_string()),
                receiver_id: user.id().clone(),
                notification: None,
            }],
        )
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [mint_payload])
        .await
        .unwrap();

    let intent = ImtBurn {
        tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
        memo: Some(memo.to_string()),
    };
    let burn_payload = user
        .sign_defuse_payload_default(&env.defuse, [intent.clone()])
        .await
        .unwrap();

    let result = env
        .simulate_and_execute_intents(env.defuse.id(), [burn_payload.clone()])
        .await
        .unwrap();

    let mt_id = TokenId::from(ImtTokenId::new(user.id().clone(), token_id.to_string()));

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &mt_id.to_string())
            .await
            .unwrap(),
        0
    );

    assert_a_contains_b!(
            a: result.logs().clone(),
            b: [
                MtEvent::MtBurn(Cow::Owned(vec![MtBurnEvent {
                owner_id: user.id().into(),
                token_ids: vec![mt_id.to_string()].into(),
                amounts: vec![U128::from(amount)].into(),
                memo: Some(memo.into()),
                authorized_id: None,
            }]))
            .to_nep297_event()
            .to_event_log(),
                DefuseEvent::ImtBurn(Cow::Owned(vec![IntentEvent {
                intent_hash: burn_payload.hash(),
                event: AccountEvent {
                    account_id: user.id().clone().into(),
                    event: Cow::Owned(ImtBurn{
                        tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
                        ..intent
                    }),
                },
            }]))
            .to_nep297_event()
            .to_event_log(),
    ]
        );

    assert_a_contains_b!(
        a: result.logs().clone(),
        b: [
            MtEvent::MtBurn(Cow::Owned(vec![MtBurnEvent {
            owner_id: user.id().into(),
            token_ids: vec![mt_id.to_string()].into(),
            amounts: vec![U128::from(amount)].into(),
            memo: Some(memo.into()),
            authorized_id: None,
        }]))
        .to_nep297_event()
        .to_event_log(),]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn failed_to_burn_tokens() {
    let env = Env::builder().build().await;

    let (user, other_user, ft) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    let memo = "Some memo";
    let amount = 1000;

    // Tokens can be burned only by minter
    {
        let withdraw_payload = user
            .sign_defuse_payload_default(
                &env.defuse,
                [ImtBurn {
                    tokens: Amounts::new(
                        std::iter::once((ft.to_string().to_string(), amount)).collect(),
                    ),
                    memo: Some(memo.to_string()),
                }],
            )
            .await
            .unwrap();

        env.simulate_and_execute_intents(env.defuse.id(), [withdraw_payload])
            .await
            .unwrap_err();
    }

    // Tokens can be burned only by minter
    {
        let token_id = "sometoken.near".to_string();

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

        let withdraw_payload = other_user
            .sign_defuse_payload_default(
                &env.defuse,
                [ImtBurn {
                    tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
                    memo: Some(memo.to_string()),
                }],
            )
            .await
            .unwrap();

        env.simulate_and_execute_intents(env.defuse.id(), [withdraw_payload])
            .await
            .unwrap_err();
    }
}
