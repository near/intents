use std::borrow::Cow;

use defuse::core::amounts::Amounts;
use defuse::core::intents::tokens::Burn;
use defuse::core::token_id::{TokenId, nep141::Nep141TokenId};
use defuse::nep245::{MtBurnEvent, MtEvent};
use defuse::sandbox_ext::intents::ExecuteIntentsExt;
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
async fn burn_intent() {
    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let token_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));
    let memo = "Some memo";
    let amount = 1000;

    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    {
        env.defuse_ft_deposit_to(ft.id(), amount, user.id(), None)
            .await
            .unwrap();

        assert_eq!(
            env.defuse
                .mt_balance_of(user.id(), &token_id.to_string())
                .await
                .unwrap(),
            amount
        );
    }

    let initial_withdraw_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [Burn {
                tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
                memo: Some(memo.to_string()),
            }],
        )
        .await
        .unwrap();

    let result = env
        .simulate_and_execute_intents(env.defuse.id(), [initial_withdraw_payload])
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &token_id.to_string())
            .await
            .unwrap(),
        0
    );

    assert_a_contains_b!(
        a: result.logs().clone(),
        b: [MtEvent::MtBurn(Cow::Owned(vec![MtBurnEvent {
            owner_id: user.id().into(),
            token_ids: vec![token_id.to_string()].into(),
            amounts: vec![U128::from(amount)].into(),
            memo: Some(memo.into()),
            authorized_id: None,
        }]))
        .to_nep297_event()
        .to_event_log(),]
    );
}
