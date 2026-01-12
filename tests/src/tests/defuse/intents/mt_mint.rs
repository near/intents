use std::borrow::Cow;

use defuse::core::accounts::AccountEvent;
use defuse::core::amounts::Amounts;
use defuse::core::crypto::Payload;
use defuse::core::events::DefuseEvent;
use defuse::core::intents::IntentEvent;
use defuse::core::intents::tokens::MtMint;
use defuse::core::token_id::{TokenId, nep141::Nep141TokenId};
use defuse::nep245::{MtEvent, MtMintEvent};
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
async fn mt_mint_intent() {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let token = TokenId::from(Nep141TokenId::new(
        "sometoken.near".parse::<AccountId>().unwrap(),
    ));
    let memo = "Some memo";
    let amount = 1000;

    let intent = MtMint {
        tokens: Amounts::new(std::iter::once((token.clone(), amount)).collect()),
        memo: Some(memo.to_string()),
    };
    let mint_payload = user
        .sign_defuse_payload_default(&env.defuse, [intent.clone()])
        .await
        .unwrap();

    let result = env
        .simulate_and_execute_intents(env.defuse.id(), [mint_payload.clone()])
        .await
        .unwrap();

    let mt_id = TokenId::from(Nep245TokenId::new(user.id().clone(), token.to_string()));

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &mt_id.to_string())
            .await
            .unwrap(),
        amount
    );

    assert_a_contains_b!(
        a: result.logs().clone(),
        b: [MtEvent::MtMint(Cow::Owned(vec![MtMintEvent {
            owner_id: user.id().into(),
            token_ids: vec![mt_id.to_string()].into(),
            amounts: vec![U128::from(amount)].into(),
            memo: Some(memo.into()),
        }]))
        .to_nep297_event()
        .to_event_log(),
            DefuseEvent::MtMint(Cow::Owned(vec![IntentEvent {
            intent_hash: mint_payload.hash(),
            event: AccountEvent {
                account_id: user.id().clone().into(),
                event: Cow::Owned(intent),
            },
        }]))
        .to_nep297_event()
        .to_event_log(),
            ]
    );
}
