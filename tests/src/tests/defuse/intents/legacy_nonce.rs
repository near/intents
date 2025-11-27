use crate::{
    tests::defuse::{DefuseSigner, SigningStandard, env::Env, intents::ExecuteIntentsExt},
    utils::mt::MtExt,
};
use defuse::core::{
    Deadline, Nonce,
    amounts::Amounts,
    intents::{DefuseIntents, tokens::Transfer},
    token_id::{TokenId, nep141::Nep141TokenId},
};
use defuse_test_utils::random::make_arbitrary;
use rstest::rstest;

#[tokio::test]
#[rstest]
#[trace]
async fn execute_intent_with_legacy_nonce(#[from(make_arbitrary)] legacy_nonce: Nonce) {
    let env = Env::builder().no_registration(true).build().await;

    let (user1, user2, ft1) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user1.id(), user2.id()], vec![&ft1])
        .await;

    env.defuse_ft_deposit_to(&ft1, 1000, user1.id(), None)
        .await
        .unwrap();

    let token_id = TokenId::from(Nep141TokenId::new(ft1.clone()));

    assert_eq!(
        env.defuse
            .mt_balance_of(user1.id(), &token_id.to_string())
            .await
            .unwrap(),
        1000
    );
    assert_eq!(
        env.defuse
            .mt_balance_of(user2.id(), &token_id.to_string())
            .await
            .unwrap(),
        0
    );

    let transfer_intent = Transfer {
        receiver_id: user2.id().clone(),
        tokens: Amounts::new(std::iter::once((token_id.clone(), 1000)).collect()),
        memo: None,
        notification: None,
    };

    let transfer_intent_payload = user1.sign_defuse_message(
        SigningStandard::default(),
        env.defuse.id(),
        legacy_nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![transfer_intent.into()],
        },
    );

    let _ = env
        .defuse
        .execute_intents(env.defuse.id(), [transfer_intent_payload])
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(user1.id(), &token_id.to_string())
            .await
            .unwrap(),
        0
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(user2.id(), &token_id.to_string())
            .await
            .unwrap(),
        1000
    );
}
