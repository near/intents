use crate::tests::defuse::env::Env;
use defuse_sandbox::extensions::{
    defuse::{
        AccountArgs, DefuseExt, DefuseSignerExt, ToEventLog,
        core::{
            DefuseError,
            accounts::AccountEvent,
            amounts::Amounts,
            events::DefuseEvent,
            intents::{MaybeIntentEvent, account::SetAuthByPredecessorId, tokens::Transfer},
            token_id::{TokenId, nep141::Nep141TokenId},
        },
    },
    mt::{Mt, MtBalanceOfArgs, MtExt},
};
use defuse_test_utils::asserts::ResultAssertsExt;
use near_sdk::{AccountId, AsNep297Event};
use rstest::rstest;
use std::borrow::Cow;

#[rstest]
#[tokio::test]
async fn auth_by_predecessor_id() {
    let env = Env::new().await;

    let (user, ft) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user.account_id()], vec![ft.contract_id()])
        .await;

    let receiver_id: AccountId = "receiver_id.near".parse().unwrap();

    // deposit tokens
    env.defuse_ft_deposit_to(ft.contract_id(), 1000, user.account_id(), None)
        .await
        .unwrap();

    let ft: TokenId = Nep141TokenId::new(ft.contract_id().clone()).into();

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user.account_id(),
                token_id: &ft.to_string(),
            })
            .await
            .unwrap()
            .0,
        1000
    );

    // disable auth by PREDECESSOR_ID
    {
        assert!(
            env.defuse
                .is_auth_by_predecessor_id_enabled(AccountArgs {
                    account_id: user.account_id(),
                })
                .await
                .unwrap()
        );

        let result = user
            .defuse_disable_auth_by_predecessor_id(env.defuse.contract_id())
            .await
            .unwrap();

        let event =
            DefuseEvent::SetAuthByPredecessorId(MaybeIntentEvent::new_fn_call(AccountEvent::new(
                user.account_id().clone(),
                Cow::Owned(SetAuthByPredecessorId { enabled: false }),
            )))
            .to_nep297_event()
            .to_event_log();

        assert_eq!(result.logs(), [event]);

        assert!(
            !env.defuse
                .is_auth_by_predecessor_id_enabled(AccountArgs {
                    account_id: user.account_id(),
                })
                .await
                .unwrap()
        );

        // second attempt should fail, since already disabled
        user.defuse_disable_auth_by_predecessor_id(env.defuse.contract_id())
            .await
            .assert_err_contains(
                DefuseError::AuthByPredecessorIdDisabled(user.account_id().clone()).to_string(),
            );
    }

    // transfer via tx should fail
    {
        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: user.account_id(),
                    token_id: &ft.to_string(),
                })
                .await
                .unwrap()
                .0,
            1000
        );

        user.mt_transfer(
            env.defuse.contract_id(),
            &receiver_id,
            &ft.to_string(),
            100,
            None,
        )
        .await
        .assert_err_contains(
            DefuseError::AuthByPredecessorIdDisabled(user.account_id().clone()).to_string(),
        );

        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: user.account_id(),
                    token_id: &ft.to_string(),
                })
                .await
                .unwrap()
                .0,
            1000
        );
        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: &receiver_id,
                    token_id: &ft.to_string(),
                })
                .await
                .unwrap()
                .0,
            0
        );
    }

    // transfer via intent should succeed
    {
        let transfer_payload = user
            .sign_defuse_payload_default(
                &env.defuse,
                [Transfer {
                    receiver_id: receiver_id.clone(),
                    tokens: Amounts::new([(ft.clone(), 200)].into()),
                    memo: None,
                    notification: None,
                }],
            )
            .await
            .unwrap();

        env.defuse_simulate_and_execute_intents(env.defuse.contract_id(), [transfer_payload])
            .await
            .unwrap();

        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: user.account_id(),
                    token_id: &ft.to_string(),
                })
                .await
                .unwrap()
                .0,
            800
        );
        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: &receiver_id,
                    token_id: &ft.to_string(),
                })
                .await
                .unwrap()
                .0,
            200
        );
    }

    // enable auth by PREDECESSOR_ID back (by intent)
    {
        let intent = SetAuthByPredecessorId { enabled: true };
        let enable_auth_payload = user
            .sign_defuse_payload_default(&env.defuse, [intent.clone()])
            .await
            .unwrap();

        let result = env
            .defuse_simulate_and_execute_intents(
                env.defuse.contract_id(),
                [enable_auth_payload.clone()],
            )
            .await
            .unwrap();

        assert_eq!(result.logs(), enable_auth_payload.to_event_log());

        assert!(
            env.defuse
                .is_auth_by_predecessor_id_enabled(AccountArgs {
                    account_id: user.account_id()
                })
                .await
                .unwrap()
        );
    }

    // transfer via tx should succeed, since auth by PREDECESSOR_ID was
    // enabled back
    {
        user.mt_transfer(
            env.defuse.contract_id(),
            &receiver_id,
            &ft.to_string(),
            400,
            None,
        )
        .await
        .unwrap();

        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: user.account_id(),
                    token_id: &ft.to_string(),
                })
                .await
                .unwrap()
                .0,
            400
        );
        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: &receiver_id,
                    token_id: &ft.to_string(),
                })
                .await
                .unwrap()
                .0,
            600
        );
    }
}
