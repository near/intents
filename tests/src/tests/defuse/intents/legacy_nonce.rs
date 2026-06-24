use defuse_sandbox::{
    assert_eq_defuse_event_logs,
    extensions::{
        defuse::{
            DefuseExt, DefuseSignerExt, ToEventLog,
            core::{
                Deadline, Nonce,
                amounts::Amounts,
                intents::{DefuseIntents, tokens::Transfer},
                token_id::{TokenId, nep141::Nep141TokenId},
            },
        },
        mt::{Mt, MtBalanceOfArgs},
    },
};

use defuse_test_utils::random::make_arbitrary;
use rstest::rstest;

use crate::tests::defuse::env::{Env, env};

#[rstest]
#[tokio::test]
async fn execute_intent_with_legacy_nonce(
    #[future(awt)] env: Env,
    #[from(make_arbitrary)] legacy_nonce: Nonce,
) {
    let (user1, user2, ft1) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(
        vec![user1.account_id(), user2.account_id()],
        vec![ft1.contract_id()],
    )
    .await;

    env.defuse_ft_deposit_to(ft1.contract_id(), 1000, user1.account_id(), None)
        .await
        .unwrap();

    let token_id = TokenId::from(Nep141TokenId::new(ft1.contract_id().clone()));

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user1.account_id(),
                token_id: &token_id.to_string()
            })
            .await
            .unwrap()
            .0,
        1000
    );
    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user2.account_id(),
                token_id: &token_id.to_string()
            })
            .await
            .unwrap()
            .0,
        0
    );

    let transfer_intent = Transfer {
        receiver_id: user2.account_id().clone(),
        tokens: Amounts::new(std::iter::once((token_id.clone(), 1000)).collect()),
        memo: None,
        notification: None,
    };

    let transfer_intent_payload = user1
        .sign_defuse_message(
            env.defuse.contract_id(),
            legacy_nonce,
            Deadline::MAX,
            DefuseIntents {
                intents: vec![transfer_intent.into()],
            },
        )
        .await;

    let (res, _) = env
        .defuse_simulate_and_execute_intents(
            env.defuse.contract_id(),
            [transfer_intent_payload.clone()],
        )
        .await
        .unwrap();

    assert_eq_defuse_event_logs!(transfer_intent_payload.to_event_log(), res.logs());

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user1.account_id(),
                token_id: &token_id.to_string()
            })
            .await
            .unwrap()
            .0,
        0
    );

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user2.account_id(),
                token_id: &token_id.to_string()
            })
            .await
            .unwrap()
            .0,
        1000
    );
}
