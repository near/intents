use defuse_sandbox::extensions::{
    defuse::{
        DefuseExt, DefuseSignerExt, ToEventLog,
        core::{
            amounts::Amounts,
            intents::imt::{ImtBurn, ImtMint},
            token_id::{TokenId, imt::ImtTokenId, nep141::Nep141TokenId},
        },
    },
    mt::{Mt, MtBalanceOfArgs},
};
use rstest::rstest;

use crate::tests::defuse::{
    env::{Env, EnvBuilder, env},
    utils::assert_eq_defuse_event_logs,
};

#[rstest]
#[tokio::test]
async fn imt_burn_intent(
    #[future(awt)]
    #[with(EnvBuilder::default().imt())]
    env: Env,
) {
    let (user, other_user) = futures::join!(env.create_user(), env.create_user());

    let token_id = "sometoken.near".to_string();
    let memo = "Some memo";
    let amount = 1000;

    let mt_id = TokenId::from(ImtTokenId::new(user.account_id().clone(), token_id.clone()));

    let mint_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [ImtMint {
                tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
                memo: Some(memo.to_string()),
                receiver_id: other_user.account_id().clone(),
                notification: None,
            }],
        )
        .await
        .unwrap();

    env.defuse_simulate_and_execute_intents(env.defuse.contract_id(), [mint_payload])
        .await
        .unwrap();

    let intent = ImtBurn {
        minter_id: user.account_id().clone(),
        tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
        memo: Some(memo.to_string()),
    };
    let burn_payload = other_user
        .sign_defuse_payload_default(&env.defuse, [intent.clone()])
        .await
        .unwrap();

    let (result, _) = env
        .defuse_simulate_and_execute_intents(env.defuse.contract_id(), [burn_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: other_user.account_id(),
                token_id: &mt_id.to_string()
            })
            .await
            .unwrap()
            .0,
        0
    );

    assert_eq_defuse_event_logs(burn_payload.to_event_log(), result.logs());
}

#[rstest]
#[tokio::test]
async fn failed_to_burn_tokens_with_intent(
    #[future(awt)]
    #[with(EnvBuilder::default().imt())]
    env: Env,
) {
    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    let memo = "Some memo";
    let amount = 1000;

    // Only minted imt tokens can be burned
    let ft_id = TokenId::from(Nep141TokenId::new(ft.contract_id()));

    let withdraw_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [ImtBurn {
                minter_id: user.account_id().clone(),
                tokens: Amounts::new(vec![(ft_id.to_string(), amount)].into_iter().collect()),
                memo: Some(memo.to_string()),
            }],
        )
        .await
        .unwrap();

    env.defuse_simulate_and_execute_intents(env.defuse.contract_id(), [withdraw_payload])
        .await
        .unwrap_err();
}
