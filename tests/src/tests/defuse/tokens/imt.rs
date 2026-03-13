use defuse::core::{amounts::Amounts, intents::imt::ImtMint, token_id::TokenId};
use defuse_escrow_swap::token_id::{imt::ImtTokenId, nep141::Nep141TokenId};
use defuse_sandbox::extensions::defuse::{
    intents::ExecuteIntentsExt, signer::DefaultDefuseSignerExt, tokens::imt::DefuseImtBurner,
};
use defuse_sandbox::extensions::mt::MtViewExt;
use rstest::rstest;

use crate::tests::defuse::env::Env;

#[rstest]
#[tokio::test]
async fn imt_burn_call() {
    let env = Env::builder().create_unique_users().build().await;

    let (user1, user2) = futures::join!(env.create_user(), env.create_user());
    let token = "sometoken.near".to_string();
    let memo = "Some memo";
    let amount = 1000;

    let imt_id = TokenId::from(ImtTokenId::new(user1.id().clone(), token.to_string()));

    // Mint tokens first
    {
        let mint_payload = user1
            .sign_defuse_payload_default(
                &env.defuse,
                [ImtMint {
                    tokens: Amounts::new(std::iter::once((token.clone(), amount)).collect()),
                    memo: Some(memo.to_string()),
                    receiver_id: user2.id().clone(),
                    notification: None,
                }],
            )
            .await
            .unwrap();

        env.simulate_and_execute_intents(env.defuse.id(), [mint_payload])
            .await
            .unwrap();

        assert_eq!(
            env.defuse
                .mt_balance_of(user2.id(), &imt_id.to_string())
                .await
                .unwrap(),
            amount
        );
    }

    user2
        .imt_burn(
            env.defuse.id(),
            user1.id().clone(),
            [(token.clone(), amount)],
            Some(memo.to_string()),
        )
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(user2.id(), &imt_id.to_string())
            .await
            .unwrap(),
        0
    );

    // TODO: compare events
}

#[rstest]
#[trace]
#[tokio::test]
async fn failed_to_burn_tokens_by_fn_call() {
    let env = Env::builder().build().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let memo = "Some memo";
    let amount = 1000;

    // Only minted imt tokens can be burned
    let ft_id = TokenId::from(Nep141TokenId::new(ft));

    user.imt_burn(
        env.defuse.id(),
        user.id().clone(),
        [(ft_id.to_string(), amount)],
        Some(memo.to_string()),
    )
    .await
    .unwrap_err();
}
