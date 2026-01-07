use defuse::core::intents::tokens::Transfer;
use defuse_tests::{
    contract_extensions::defuse::intents::ExecuteIntentsExt, defuse_signer::DefuseSignerExt,
    env::Env, sandbox::extensions::mt::MtViewExt,
};

use defuse::core::token_id::{TokenId, nep141::Nep141TokenId};

use near_sdk::AccountId;
use rstest::rstest;

use defuse::core::amounts::Amounts;

#[rstest]
#[trace]
#[tokio::test]
async fn transfer_intent() {
    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let other_user_id: AccountId = "other-user.near".parse().unwrap();
    let token_id = TokenId::from(Nep141TokenId::new(ft.id().clone()));

    env.initial_ft_storage_deposit(vec![user.id()], vec![ft.id()])
        .await;

    env.defuse_ft_deposit_to(ft.id(), 1000, user.id(), None)
        .await
        .unwrap();

    let transfer_intent = Transfer {
        receiver_id: other_user_id.clone(),
        tokens: Amounts::new(
            std::iter::once((TokenId::from(Nep141TokenId::new(ft.id().clone())), 1000)).collect(),
        ),
        memo: None,
        notification: None,
    };

    let initial_transfer_payload = user
        .sign_defuse_payload_default(&env.defuse, [transfer_intent])
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [initial_transfer_payload])
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &token_id.to_string())
            .await
            .unwrap(),
        0
    );

    assert_eq!(
        env.defuse
            .mt_balance_of(&other_user_id, &token_id.to_string())
            .await
            .unwrap(),
        1000
    );
}
