use crate::{tests::defuse::DefuseSignerExt, utils::fixtures::public_key};
use defuse::{
    contract::Role,
    core::{
        DefuseError,
        crypto::PublicKey,
        intents::Intent,
        token_id::{TokenId, nep141::Nep141TokenId},
    },
};

use defuse_test_utils::asserts::ResultAssertsExt;
use rstest::rstest;

use crate::{
    tests::defuse::{
        
        accounts::{AccountManagerExt, traits::ForceAccountManagerExt},
        env::Env,
        intents::ExecuteIntentsExt,
        tokens::nep141::traits::DefuseFtWithdrawer,
    },
    utils::{acl::AclExt, mt::MtExt, payload::ExtractNonceExt},
};

#[tokio::test]
#[rstest]
async fn test_lock_account(public_key: PublicKey) {
    let env = Env::builder().deployer_as_super_admin().build().await;

    let (locked_account, account_locker, unlocked_account, ft) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_user(),
        env.create_token()
    );

    env.initial_ft_storage_deposit(vec![locked_account.id(), unlocked_account.id()], vec![&ft])
        .await;

    // deposit tokens
    let ft1: TokenId = Nep141TokenId::new(ft.clone()).into();
    {
        env.defuse_ft_deposit_to(&ft, 1000, locked_account.id())
            .await
            .unwrap();

        env.defuse_ft_deposit_to(&ft, 3000, unlocked_account.id())
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
        locked_account
            .add_public_key(env.defuse.id(), public_key)
            .await
            .assert_err_contains(
                DefuseError::AccountLocked(locked_account.id().clone()).to_string(),
            );

        assert!(
            !env.defuse
                .has_public_key(locked_account.id(), &public_key)
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
            .assert_err_contains(
                DefuseError::AccountLocked(locked_account.id().clone()).to_string(),
            );

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
                .defuse_ft_withdraw(env.defuse.id(), unlocked_account.id(), &ft, 100, None, msg)
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
        env.defuse_ft_deposit_to(&ft, 100, locked_account.id())
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
        let locked_payload = locked_account
            .sign_defuse_payload_default(env.defuse.id(), Vec::<Intent>::new())
            .await
            .unwrap();
        let nonce = locked_payload.extract_nonce().unwrap();

        env.defuse
            .execute_intents(env.defuse.id(), [locked_payload])
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

#[tokio::test]
#[rstest]
async fn test_force_set_auth_by_predecessor_id(public_key: PublicKey) {
    let env = Env::builder().deployer_as_super_admin().build().await;

    let (user_account, account_locker, account_unlocker) =
        futures::join!(env.create_user(), env.create_user(), env.create_user());

    // disable auth by predecessor id
    {
        // no permisson
        {
            account_locker
                .force_disable_auth_by_predecessor_ids(env.defuse.id(), [user_account.id().clone()])
                .await
                .expect_err(&format!(
                    "{} doesn't have {:?} role yet",
                    account_locker.id(),
                    Role::UnrestrictedAccountLocker,
                ));
            assert!(
                env.defuse
                    .is_auth_by_predecessor_id_enabled(user_account.id())
                    .await
                    .unwrap()
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

        // permisson granted
        {
            account_locker
                .force_disable_auth_by_predecessor_ids(env.defuse.id(), [user_account.id().clone()])
                .await
                .unwrap();
            assert!(
                !env.defuse
                    .is_auth_by_predecessor_id_enabled(user_account.id())
                    .await
                    .unwrap()
            );
        }
    }

    // try to execute tx from user's account with disabled auth by predecessor id
    {
        user_account
            .add_public_key(env.defuse.id(), public_key)
            .await
            .unwrap_err();
        assert!(
            !env.defuse
                .has_public_key(user_account.id(), &public_key)
                .await
                .unwrap()
        );
    }

    // enable auth by predecessor id
    {
        // no permisson
        {
            account_unlocker
                .force_enable_auth_by_predecessor_ids(env.defuse.id(), [user_account.id().clone()])
                .await
                .expect_err(&format!(
                    "{} doesn't have {:?} role yet",
                    account_unlocker.id(),
                    Role::UnrestrictedAccountUnlocker,
                ));
            assert!(
                !env.defuse
                    .is_auth_by_predecessor_id_enabled(user_account.id())
                    .await
                    .unwrap()
            );
        }

        // grant UnrestrictedAccountUnlocker role
        env.acl_grant_role(
            env.defuse.id(),
            Role::UnrestrictedAccountUnlocker,
            account_unlocker.id(),
        )
        .await
        .unwrap();

        // permisson granted
        {
            account_unlocker
                .force_enable_auth_by_predecessor_ids(env.defuse.id(), [user_account.id().clone()])
                .await
                .unwrap();
            assert!(
                env.defuse
                    .is_auth_by_predecessor_id_enabled(user_account.id())
                    .await
                    .unwrap()
            );
        }
    }

    // try to execute tx from user's account with enabled auth by predecessor id
    {
        user_account
            .add_public_key(env.defuse.id(), public_key)
            .await
            .unwrap();
        assert!(
            env.defuse
                .has_public_key(user_account.id(), &public_key)
                .await
                .unwrap()
        );
    }
}
