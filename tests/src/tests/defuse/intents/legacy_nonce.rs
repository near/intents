use defuse::core::{amounts::Amounts, intents::{tokens::Transfer, DefuseIntents}, token_id::{nep141::Nep141TokenId, TokenId}, Deadline, Nonce};
use crate::{tests::defuse::{env::Env, intents::ExecuteIntentsExt, DefuseSigner, SigningStandard}, utils::mt::MtExt};
use defuse_test_utils::random::{make_arbitrary};
use rstest::rstest;

#[tokio::test]
#[rstest]
#[trace]
async fn execute_intent_with_legacy_nonce(
    #[from(make_arbitrary)] legacy_nonce: Nonce,
) {
    let env = Env::builder().no_registration(true).build().await;

    let (user1, user2, ft1) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user1.id(), user2.id()], vec![&ft1])
        .await;

    env.defuse_ft_deposit_to(&ft1, 1000, user1.id())
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(user1.id(), &ft1.to_string())
            .await
            .unwrap(),
        1000
    );
    assert_eq!(
        env.defuse
            .mt_balance_of(user2.id(), &ft1.to_string())
            .await
            .unwrap(),
        0
    );

    let transfer_intent = Transfer {
        receiver_id: user2.id().clone(),
        tokens: Amounts::new(
            std::iter::once((TokenId::from(Nep141TokenId::new(ft1.clone())), 1000)).collect(),
        ),
        memo: None,
    };

    let transfer_intent_payload = user1
        .sign_defuse_message(SigningStandard::default(), env.defuse.id(), legacy_nonce, Deadline::MAX, DefuseIntents{intents: vec![transfer_intent.into()]});

    let result = env
        .defuse
        .execute_intents(env.defuse.id(), [transfer_intent_payload])
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(user1.id(), &ft1.to_string())
            .await
            .unwrap(),
        0
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(user2.id(), &ft1.to_string())
            .await
            .unwrap(),
        1000
    );


}
