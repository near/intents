use std::time::Duration;

use arbitrary::Unstructured;
use defuse::core::{
    Deadline, DefuseError,
    amounts::Amounts,
    intents::{DefuseIntents, account::SetAuthByPredecessorId, tokens::Transfer},
    token_id::{TokenId, nep141::Nep141TokenId},
};
use defuse_test_utils::{asserts::ResultAssertsExt, random::random_bytes};
use rstest::rstest;

use crate::{
    tests::defuse::{
        DefuseSigner, accounts::AccountManagerExt, env::Env, intents::ExecuteIntentsExt,
    },
    utils::mt::MtExt,
};

#[tokio::test]
#[rstest]
async fn test(random_bytes: Vec<u8>) {
    let mut u = Unstructured::new(&random_bytes);
    let env = Env::new().await;

    let ft1: TokenId = Nep141TokenId::new(env.ft1.clone()).into();
    // deposit tokens
    env.defuse_ft_deposit_to(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(env.user1.id(), &ft1.to_string())
            .await
            .unwrap(),
        1000
    );

    // disable auth by PREDECESSOR_ID
    {
        assert!(
            env.defuse
                .is_auth_by_predecessor_id_enabled(env.user1.id())
                .await
                .unwrap()
        );
        env.user1
            .disable_auth_by_predecessor_id(env.defuse.id())
            .await
            .unwrap();

        assert!(
            !env.defuse
                .is_auth_by_predecessor_id_enabled(env.user1.id())
                .await
                .unwrap()
        );

        // second attempt should fail, since already disabled
        env.user1
            .disable_auth_by_predecessor_id(env.defuse.id())
            .await
            .assert_err_contains(
                DefuseError::AuthByPredecessorIdDisabled(env.user1.id().clone()).to_string(),
            );
    }

    // transfer via tx should fail
    {
        assert_eq!(
            env.defuse
                .mt_balance_of(env.user1.id(), &ft1.to_string())
                .await
                .unwrap(),
            1000
        );

        env.user1
            .mt_transfer(
                env.defuse.id(),
                env.user2.id(),
                &ft1.to_string(),
                100,
                None,
                None,
            )
            .await
            .assert_err_contains(
                DefuseError::AuthByPredecessorIdDisabled(env.user1.id().clone()).to_string(),
            );

        assert_eq!(
            env.defuse
                .mt_balance_of(env.user1.id(), &ft1.to_string())
                .await
                .unwrap(),
            1000
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(env.user2.id(), &ft1.to_string())
                .await
                .unwrap(),
            0
        );
    }

    // transfer via intent should succeed
    {
        env.defuse
            .execute_intents([env.user1.sign_defuse_message(
                u.arbitrary().unwrap(),
                env.defuse.id(),
                u.arbitrary().unwrap(),
                Deadline::timeout(Duration::from_secs(120)),
                DefuseIntents {
                    intents: [Transfer {
                        receiver_id: env.user2.id().clone(),
                        tokens: Amounts::new([(ft1.clone(), 200)].into()),
                        memo: None,
                    }
                    .into()]
                    .into(),
                },
            )])
            .await
            .unwrap();

        assert_eq!(
            env.defuse
                .mt_balance_of(env.user1.id(), &ft1.to_string())
                .await
                .unwrap(),
            800
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(env.user2.id(), &ft1.to_string())
                .await
                .unwrap(),
            200
        );
    }

    // enable auth by PREDECESSOR_ID back (by intent)
    {
        env.defuse
            .execute_intents([env.user1.sign_defuse_message(
                u.arbitrary().unwrap(),
                env.defuse.id(),
                u.arbitrary().unwrap(),
                Deadline::timeout(Duration::from_secs(120)),
                DefuseIntents {
                    intents: [SetAuthByPredecessorId { enabled: true }.into()].into(),
                },
            )])
            .await
            .unwrap();

        assert!(
            env.defuse
                .is_auth_by_predecessor_id_enabled(env.user1.id())
                .await
                .unwrap()
        );
    }

    // transfer via tx should succeed, since auth by PREDECESSOR_ID was
    // enabled back
    {
        env.user1
            .mt_transfer(
                env.defuse.id(),
                env.user2.id(),
                &ft1.to_string(),
                400,
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(
            env.defuse
                .mt_balance_of(env.user1.id(), &ft1.to_string())
                .await
                .unwrap(),
            400
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(env.user2.id(), &ft1.to_string())
                .await
                .unwrap(),
            600
        );
    }
}
