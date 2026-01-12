use std::borrow::Cow;

use defuse::core::accounts::AccountEvent;
use defuse::core::amounts::Amounts;
use defuse::core::crypto::Payload;
use defuse::core::events::DefuseEvent;
use defuse::core::intents::tokens::{MtBurn, MtMint, Transfer};
use defuse::core::intents::{Intent, IntentEvent};
use defuse::core::token_id::{TokenId, nep141::Nep141TokenId};
use defuse::nep245::{MtBurnEvent, MtEvent};
use defuse::sandbox_ext::intents::ExecuteIntentsExt;
use defuse_escrow_swap::token_id::nep245::Nep245TokenId;
use defuse_sandbox::assert_a_contains_b;
use defuse_sandbox::extensions::mt::MtViewExt;
use near_sdk::json_types::U128;
use rstest::rstest;

use near_sdk::{AccountId, AsNep297Event};

use crate::tests::defuse::DefuseSignerExt;
use crate::tests::defuse::env::Env;

#[rstest]
#[trace]
#[tokio::test]
async fn mt_burn_intent() {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let token_id = TokenId::from(Nep141TokenId::new(
        "sometoken.near".parse::<AccountId>().unwrap(),
    ));
    let memo = "Some memo";
    let amount = 1000;

    let mint_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [MtMint {
                tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
                memo: Some(memo.to_string()),
            }],
        )
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [mint_payload])
        .await
        .unwrap();

    let intent = MtBurn {
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

    let mt_id = TokenId::from(Nep245TokenId::new(user.id().clone(), token_id.to_string()));

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
                DefuseEvent::MtBurn(Cow::Owned(vec![IntentEvent {
                intent_hash: burn_payload.hash(),
                event: AccountEvent {
                    account_id: user.id().clone().into(),
                    event: Cow::Owned(intent),
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
        let token_id = TokenId::from(Nep141TokenId::new(ft.id()));
        let withdraw_payload = user
            .sign_defuse_payload_default(
                &env.defuse,
                [MtBurn {
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

    // Tokens can be burned only by minter
    {
        let token_id = TokenId::from(Nep141TokenId::new(
            "sometoken.near".parse::<AccountId>().unwrap(),
        ));

        let mint_payload = user
            .sign_defuse_payload_default(
                &env.defuse,
                [
                    Intent::from(MtMint {
                        tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
                        memo: Some(memo.to_string()),
                    }),
                    Transfer {
                        receiver_id: other_user.id().clone(),
                        tokens: Amounts::new(
                            std::iter::once((
                                TokenId::from(Nep245TokenId::new(
                                    user.id().clone(),
                                    token_id.to_string(),
                                )),
                                amount,
                            ))
                            .collect(),
                        ),
                        memo: None,
                        notification: None,
                    }
                    .into(),
                ],
            )
            .await
            .unwrap();

        env.simulate_and_execute_intents(env.defuse.id(), [mint_payload])
            .await
            .unwrap();

        let withdraw_payload = other_user
            .sign_defuse_payload_default(
                &env.defuse,
                [MtBurn {
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
