use crate::tests::defuse::DefuseSignerExt;
use crate::tests::defuse::env::Env;
use crate::utils::fixtures::{ed25519_pk, p256_pk, secp256k1_pk};
use defuse::extensions::account_manager::{AccountManagerExt, AccountViewExt};
use defuse::extensions::deployer::DEFUSE_WASM;
use defuse::extensions::intents::ExecuteIntentsExt;
use defuse::extensions::state::{FeesManagerExt, FeesManagerViewExt, SaltManagerExt, SaltViewExt};
use defuse::{
    contract::Role,
    core::{
        amounts::Amounts,
        crypto::PublicKey,
        fees::Pips,
        intents::tokens::Transfer,
        token_id::{TokenId, nep141::Nep141TokenId},
    },
    nep245::Token,
};
use defuse_sandbox::extensions::acl::AclExt;
use defuse_sandbox::extensions::mt::MtViewExt;
use defuse_sandbox::{Account, Sandbox, SigningAccount};
use itertools::Itertools;
use rstest::rstest;

use futures::future::try_join_all;

#[ignore = "only for simple upgrades"]
#[tokio::test]
#[rstest]
async fn upgrade(ed25519_pk: PublicKey, secp256k1_pk: PublicKey, p256_pk: PublicKey) {
    let sandbox = Sandbox::new().await;
    let root = sandbox.root();
    let contract = SigningAccount::new(
        Account::new(
            "intents.near".parse().unwrap(),
            root.network_config().clone(),
        ),
        root.private_key().clone(),
    );

    sandbox
        .import_contract(contract.id(), "https://nearrpc.aurora.dev")
        .await
        .unwrap();

    contract
        .tx(contract.id().clone())
        .deploy(DEFUSE_WASM.to_vec())
        .await
        .unwrap();

    assert_eq!(
        contract
            .mt_balance_of(
                &"user.near".parse().unwrap(),
                &"non-existent-token".to_string(),
            )
            .await
            .unwrap(),
        0
    );

    for public_key in [ed25519_pk, secp256k1_pk, p256_pk] {
        assert!(
            contract
                .has_public_key(&public_key.to_implicit_account_id(), &public_key)
                .await
                .unwrap()
        );

        assert!(
            !contract
                .has_public_key(contract.id(), &public_key)
                .await
                .unwrap()
        );
    }
}

#[rstest]
#[tokio::test]
async fn test_upgrade_with_persistence() {
    // initialize with persistent state and migration from legacy
    let env = Env::builder().build_with_migration().await;

    // Make some changes existing users + create new users and token
    let (user1, user2, user3, user4, ft1) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_named_user("first_new_user"),
        env.create_named_user("second_new_user"),
        env.create_token()
    );

    let existing_tokens = env.defuse.mt_tokens(..).await.unwrap();

    // Check users
    {
        env.initial_ft_storage_deposit(
            vec![user1.id(), user2.id(), user3.id(), user4.id()],
            vec![ft1.id()],
        )
        .await;

        let users = [&user1, &user2, &user3, &user4];

        // Additional deposits to new users
        try_join_all(
            users
                .iter()
                .map(|user| env.defuse_ft_deposit_to(ft1.id(), 10_000, user.id(), None)),
        )
        .await
        .expect("Failed to deposit to users");

        // Interactions between new and old users
        {
            let payloads =
                futures::future::try_join_all(users.iter().combinations(2).map(|accounts| {
                    let sender = accounts[0];
                    let receiver = accounts[1];
                    sender.sign_defuse_payload_default(
                        &env.defuse,
                        [Transfer {
                            receiver_id: receiver.id().clone(),
                            tokens: Amounts::new(
                                [(TokenId::Nep141(Nep141TokenId::new(ft1.id().clone())), 1000)]
                                    .into(),
                            ),
                            memo: None,
                            notification: None,
                        }],
                    )
                }))
                .await
                .unwrap();

            env.simulate_and_execute_intents(env.defuse.id(), payloads)
                .await
                .unwrap();
        }

        // Check auth_by_predecessor
        {
            // On old user
            user1
                .disable_auth_by_predecessor_id(env.defuse.id())
                .await
                .unwrap();

            assert!(
                !env.defuse
                    .is_auth_by_predecessor_id_enabled(user1.id())
                    .await
                    .unwrap()
            );

            // On new user
            user3
                .disable_auth_by_predecessor_id(env.defuse.id())
                .await
                .unwrap();

            assert!(
                !env.defuse
                    .is_auth_by_predecessor_id_enabled(user3.id())
                    .await
                    .unwrap()
            );
        }
    }

    // Check tokens
    {
        let tokens = env.defuse.mt_tokens(..).await.unwrap();

        // New token
        let expected: Vec<_> = existing_tokens
            .clone()
            .into_iter()
            .chain(std::iter::once(Token {
                token_id: TokenId::Nep141(Nep141TokenId::new(ft1.id().clone())).to_string(),
                owner_id: None,
            }))
            .collect();

        assert_eq!(tokens, expected);
    }

    // Check fee
    {
        let fee = Pips::from_pips(100).unwrap();

        env.acl_grant_role(env.defuse.id(), Role::FeesManager, user1.id())
            .await
            .expect("failed to grant role");

        user1
            .set_fee(env.defuse.id(), fee)
            .await
            .expect("unable to set fee");

        let current_fee = env.defuse.fee().await.unwrap();

        assert_eq!(current_fee, fee);
    }

    // Check salts
    {
        env.acl_grant_role(env.defuse.id(), Role::SaltManager, user1.id())
            .await
            .expect("failed to grant role");

        let new_salt = user1
            .update_current_salt(env.defuse.id())
            .await
            .expect("unable to rotate salt");

        let current_salt = env.defuse.current_salt().await.unwrap();

        assert_eq!(new_salt, current_salt);
    }
}
