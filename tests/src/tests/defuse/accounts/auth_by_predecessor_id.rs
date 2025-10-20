use std::time::Duration;

use arbitrary::Unstructured;
use defuse::core::{
    Deadline, DefuseError,
    amounts::Amounts,
    intents::{DefuseIntents, account::SetAuthByPredecessorId, tokens::Transfer},
    token_id::{TokenId, nep141::Nep141TokenId},
};
use defuse_test_utils::{asserts::ResultAssertsExt, random::random_bytes};
use near_sdk::AccountId;
use rstest::rstest;

use crate::{
    tests::defuse::{
        DefuseSigner, accounts::AccountManagerExt, env::Env, intents::ExecuteIntentsExt,
    },
    utils::mt::MtExt,
};

#[tokio::test]
#[rstest]
async fn auth_by_predecessor_id(random_bytes: Vec<u8>) {
    let mut u = Unstructured::new(&random_bytes);
    let mut env = Env::new().await;

    let ft = env.create_token("ft").await;
    let user = env.get_or_create_user().await;

    env.ft_storage_deposit_for_users(vec![user.id()], &[&ft])
        .await;
    env.ft_deposit_to_root(&[&ft]).await;

    let receiver_id: AccountId = "receiver_id.near".parse().unwrap();

    // deposit tokens
    env.defuse_ft_deposit_to(&ft, 1000, user.id())
        .await
        .unwrap();

    let ft: TokenId = Nep141TokenId::new(ft.clone()).into();

    assert_eq!(
        env.defuse
            .mt_balance_of(user.id(), &ft.to_string())
            .await
            .unwrap(),
        1000
    );

    // disable auth by PREDECESSOR_ID
    {
        assert!(
            env.defuse
                .is_auth_by_predecessor_id_enabled(user.id())
                .await
                .unwrap()
        );

        user.disable_auth_by_predecessor_id(env.defuse.id())
            .await
            .unwrap();

        assert!(
            !env.defuse
                .is_auth_by_predecessor_id_enabled(user.id())
                .await
                .unwrap()
        );

        // second attempt should fail, since already disabled
        user.disable_auth_by_predecessor_id(env.defuse.id())
            .await
            .assert_err_contains(
                DefuseError::AuthByPredecessorIdDisabled(user.id().clone()).to_string(),
            );
    }

    // transfer via tx should fail
    {
        assert_eq!(
            env.defuse
                .mt_balance_of(user.id(), &ft.to_string())
                .await
                .unwrap(),
            1000
        );

        user.mt_transfer(
            env.defuse.id(),
            &receiver_id,
            &ft.to_string(),
            100,
            None,
            None,
        )
        .await
        .assert_err_contains(
            DefuseError::AuthByPredecessorIdDisabled(user.id().clone()).to_string(),
        );

        assert_eq!(
            env.defuse
                .mt_balance_of(user.id(), &ft.to_string())
                .await
                .unwrap(),
            1000
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(&receiver_id, &ft.to_string())
                .await
                .unwrap(),
            0
        );
    }

    // transfer via intent should succeed
    {
        env.defuse
            .execute_intents([user.sign_defuse_message(
                u.arbitrary().unwrap(),
                env.defuse.id(),
                u.arbitrary().unwrap(),
                Deadline::timeout(Duration::from_secs(120)),
                DefuseIntents {
                    intents: [Transfer {
                        receiver_id: receiver_id.clone(),
                        tokens: Amounts::new([(ft.clone(), 200)].into()),
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
                .mt_balance_of(user.id(), &ft.to_string())
                .await
                .unwrap(),
            800
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(&receiver_id, &ft.to_string())
                .await
                .unwrap(),
            200
        );
    }

    // enable auth by PREDECESSOR_ID back (by intent)
    {
        env.defuse
            .execute_intents([user.sign_defuse_message(
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
                .is_auth_by_predecessor_id_enabled(user.id())
                .await
                .unwrap()
        );
    }

    // transfer via tx should succeed, since auth by PREDECESSOR_ID was
    // enabled back
    {
        user.mt_transfer(
            env.defuse.id(),
            &receiver_id,
            &ft.to_string(),
            400,
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(
            env.defuse
                .mt_balance_of(user.id(), &ft.to_string())
                .await
                .unwrap(),
            400
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(&receiver_id, &ft.to_string())
                .await
                .unwrap(),
            600
        );
    }
}
