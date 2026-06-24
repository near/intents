use defuse_sandbox::extensions::{
    defuse::{
        DefuseExt, DefuseImtExt, DefuseSignerExt,
        core::{
            amounts::Amounts,
            intents::imt::ImtMint,
            token_id::{TokenId, imt::ImtTokenId, nep141::Nep141TokenId},
        },
    },
    mt::{Mt, MtBalanceOfArgs},
};
use rstest::rstest;

use crate::tests::defuse::env::{Env, env};

#[rstest]
#[tokio::test]
async fn imt_burn_call(#[future(awt)] env: Env) {
    let (user1, user2) = futures::join!(env.create_user(), env.create_user());
    let token = "sometoken.near".to_string();
    let memo = "Some memo";
    let amount = 1000;

    let imt_id = TokenId::from(ImtTokenId::new(user1.account_id().clone(), token.clone()));

    // Mint tokens first
    {
        let mint_payload = user1
            .sign_defuse_payload_default(
                &env.defuse,
                [ImtMint {
                    tokens: Amounts::new(std::iter::once((token.clone(), amount)).collect()),
                    memo: Some(memo.to_string()),
                    receiver_id: user2.account_id().clone(),
                    notification: None,
                }],
            )
            .await
            .unwrap();

        env.defuse_simulate_and_execute_intents(env.defuse.contract_id(), [mint_payload])
            .await
            .unwrap();

        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: user2.account_id(),
                    token_id: &imt_id.to_string(),
                })
                .await
                .unwrap()
                .0,
            amount
        );
    }

    user2
        .defuse_imt_burn(
            env.defuse.contract_id(),
            user1.account_id().clone(),
            [(token.clone(), amount)],
            Some(memo.to_string()),
        )
        .await
        .unwrap();

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user2.account_id(),
                token_id: &imt_id.to_string(),
            })
            .await
            .unwrap()
            .0,
        0
    );

    // TODO: compare events
}

#[rstest]
#[tokio::test]
async fn failed_to_burn_tokens_by_fn_call(#[future(awt)] env: Env) {
    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let memo = "Some memo";
    let amount = 1000;

    // Only minted imt tokens can be burned
    let ft_id = TokenId::from(Nep141TokenId::new(ft.contract_id()));

    user.defuse_imt_burn(
        env.defuse.contract_id(),
        user.account_id().clone(),
        [(ft_id.to_string(), amount)],
        Some(memo.to_string()),
    )
    .await
    .unwrap_err();
}
