use std::time::Duration;

use arbitrary::Unstructured;
use defuse::{
    contract::Role,
    core::{
        Deadline, DefuseError, Nonce,
        crypto::PublicKey,
        intents::DefuseIntents,
        token_id::{TokenId, nep141::Nep141TokenId},
    },
};

use defuse_test_utils::{asserts::ResultAssertsExt, random::random_bytes};
use rstest::rstest;

use crate::{
    tests::defuse::{
        DefuseSigner, SigningStandard,
        accounts::{AccountManagerExt, traits::AccountForceLockerExt},
        env::Env,
        intents::ExecuteIntentsExt,
        tokens::nep141::traits::DefuseFtWithdrawer,
    },
    utils::{acl::AclExt, mt::MtExt},
};

#[tokio::test]
#[rstest]
async fn test_lock_account(random_bytes: Vec<u8>) {
    let mut u = Unstructured::new(&random_bytes);

    let env = Env::builder().deployer_as_super_admin().build().await;

    let locked_account = &env.user1;
    let account_locker = &env.user2;
    let unlocked_account = &env.user3;

    // deposit tokens
    let ft1: TokenId = Nep141TokenId::new(env.ft1.clone()).into();
    {
        env.defuse_ft_deposit_to(&env.ft1, 1000, locked_account.id())
            .await
            .unwrap();

        env.defuse_ft_deposit_to(&env.ft1, 3000, unlocked_account.id())
            .await
            .unwrap();
    }

    // lock account
    {
        // no permission
        {
            account_locker
                .force_lock_account(env.defuse.id(), locked_account.id())
                .await
                .expect_err("user2 doesn't have UnrestrictedAccountLocker role");
            assert!(
                !env.is_account_locked(env.defuse.id(), locked_account.id())
                    .await
                    .unwrap(),
                "account shouldn't be locked after failed attempt to lock it",
            );
        }

        // grant UnrestrictedAccountLocker role
        env.acl_grant_role(
            env.defuse.id(),
            Role::UnrestrictedAccountLocker,
            account_locker.id(),
        )
        .await
        .unwrap();

        // force lock account
        {
            assert!(
                account_locker
                    .force_lock_account(env.defuse.id(), locked_account.id())
                    .await
                    .expect("user2 should be able to lock an account")
            );

            assert!(
                env.is_account_locked(env.defuse.id(), locked_account.id())
                    .await
                    .unwrap(),
                "account should be locked",
            );
        }

        // force lock account, second attempt
        {
            assert!(
                !account_locker
                    .force_lock_account(env.defuse.id(), locked_account.id())
                    .await
                    .expect("locking already locked account shouldn't fail")
            );
            assert!(
                env.is_account_locked(env.defuse.id(), locked_account.id())
                    .await
                    .unwrap(),
                "account should be locked",
            );
        }
    }

    assert_eq!(
        env.defuse
            .mt_balance_of(locked_account.id(), &ft1.to_string())
            .await
            .unwrap(),
        1000
    );

    // try to add public key to locked account
    {
        let pk: PublicKey = u.arbitrary().unwrap();
        locked_account
            .add_public_key(env.defuse.id(), pk)
            .await
            .assert_err_contains(DefuseError::AccountLocked(env.user1.id().clone()).to_string());
        assert!(
            !env.defuse
                .has_public_key(locked_account.id(), &pk)
                .await
                .unwrap()
        );
    }

    // try to remove existing public key from locked account
    {
        let locked_pk: PublicKey = locked_account
            .secret_key()
            .public_key()
            .to_string()
            .parse()
            .unwrap();
        locked_account
            .remove_public_key(env.defuse.id(), locked_pk)
            .await
            .assert_err_contains(DefuseError::AccountLocked(env.user1.id().clone()).to_string());
        assert!(
            env.defuse
                .has_public_key(locked_account.id(), &locked_pk)
                .await
                .unwrap()
        );
    }

    // transfer attempt from locked account
    {
        locked_account
            .mt_transfer(
                env.defuse.id(),
                unlocked_account.id(),
                &ft1.to_string(),
                100,
                None,
                None,
            )
            .await
            .expect_err("locked account shouldn't be able to transfer");

        locked_account
            .mt_transfer_call(
                env.defuse.id(),
                unlocked_account.id(),
                &ft1.to_string(),
                100,
                None,
                None,
                String::new(),
            )
            .await
            .expect_err("locked account shouldn't be able to transfer");
    }

    // withdraw attempt from locked account
    {
        for msg in [None, Some(String::new())] {
            locked_account
                .defuse_ft_withdraw(
                    env.defuse.id(),
                    unlocked_account.id(),
                    &env.ft1,
                    100,
                    None,
                    msg,
                )
                .await
                .expect_err("locked account shouldn't be able to withdraw");
        }
    }

    assert_eq!(
        env.defuse
            .mt_balance_of(locked_account.id(), &ft1.to_string())
            .await
            .unwrap(),
        1000,
        "nothing should be transferred/withdrawn from locked account"
    );

    // deposit to locked account
    {
        env.defuse_ft_deposit_to(&env.ft1, 100, locked_account.id())
            .await
            .expect("deposits to locked account should be allowed");

        assert_eq!(
            env.defuse
                .mt_balance_of(locked_account.id(), &ft1.to_string())
                .await
                .unwrap(),
            1000 + 100
        );
    }

    // mt_transfer to locked account
    {
        unlocked_account
            .mt_transfer(
                env.defuse.id(),
                locked_account.id(),
                &ft1.to_string(),
                200,
                None,
                None,
            )
            .await
            .expect("incoming transfers to locked account should be allowed");

        assert_eq!(
            env.defuse
                .mt_balance_of(locked_account.id(), &ft1.to_string())
                .await
                .unwrap(),
            1000 + 100 + 200
        );
    }

    // mt_transfer_call to locked account
    {
        assert_eq!(
            unlocked_account
                .mt_transfer_call(
                    env.defuse.id(),
                    locked_account.id(),
                    &ft1.to_string(),
                    200,
                    None,
                    None,
                    String::new(),
                )
                .await
                .expect("incoming transfers to locked account should be allowed"),
            vec![0],
        );

        assert_eq!(
            env.defuse
                .mt_balance_of(unlocked_account.id(), &ft1.to_string())
                .await
                .unwrap(),
            3000 - 200,
            "sender balance shouldn't change"
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(locked_account.id(), &ft1.to_string())
                .await
                .unwrap(),
            1000 + 100 + 200
        );
    }

    // try to execute intents on behalf of locked account
    {
        let nonce: Nonce = u.arbitrary().unwrap();
        env.defuse
            .execute_intents([env.user1.sign_defuse_message(
                SigningStandard::Nep413,
                env.defuse.id(),
                nonce,
                Deadline::timeout(Duration::from_secs(120)),
                DefuseIntents { intents: [].into() },
            )])
            .await
            .assert_err_contains(
                DefuseError::AccountLocked(locked_account.id().clone()).to_string(),
            );

        assert!(
            !env.defuse
                .is_nonce_used(locked_account.id(), &nonce)
                .await
                .unwrap()
        );
    }

    // unlock
    {
        // no permission
        {
            account_locker
                .force_unlock_account(env.defuse.id(), locked_account.id())
                .await
                .expect_err("user2 doesn't have UnrestrictedAccountUnlocker role");
            assert!(
                env.is_account_locked(env.defuse.id(), locked_account.id())
                    .await
                    .unwrap(),
                "account should still be locked after failed attempt to unlock it",
            );
        }

        // grant UnrestrictedAccountLocker role
        env.acl_grant_role(
            env.defuse.id(),
            Role::UnrestrictedAccountUnlocker,
            account_locker.id(),
        )
        .await
        .unwrap();

        // force unlock account
        {
            assert!(
                account_locker
                    .force_unlock_account(env.defuse.id(), locked_account.id())
                    .await
                    .expect("user2 should be able to lock an account")
            );

            assert!(
                !env.is_account_locked(env.defuse.id(), locked_account.id())
                    .await
                    .unwrap(),
                "account should be unlocked",
            );
        }
    }

    // transfer from unlocked
    {
        locked_account
            .mt_transfer(
                env.defuse.id(),
                unlocked_account.id(),
                &ft1.to_string(),
                50,
                None,
                None,
            )
            .await
            .expect("account is now unlocked and outgoing transfers should be allowed");
        assert_eq!(
            env.defuse
                .mt_balance_of(locked_account.id(), &ft1.to_string())
                .await
                .unwrap(),
            1000 + 100 + 200 - 50
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(unlocked_account.id(), &ft1.to_string())
                .await
                .unwrap(),
            3000 - 200 + 50
        );
    }
}
