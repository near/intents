use super::DEFUSE_WASM;

use crate::tests::defuse::DefusePayloadBuilder;
use crate::tests::defuse::accounts::AccountManagerExt;
use crate::utils::fixtures::{ed25519_pk, p256_pk, secp256k1_pk};
use crate::{
    tests::defuse::{
        env::Env,
        intents::ExecuteIntentsExt,
        state::{FeesManagerExt, SaltManagerExt},
    },
    utils::{acl::AclExt, mt::MtExt},
};
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
use itertools::Itertools;
use near_sdk::AccountId;
use rstest::rstest;

use futures::future::try_join_all;

#[ignore = "only for simple upgrades"]
#[tokio::test]
#[rstest]
async fn upgrade(ed25519_pk: PublicKey, secp256k1_pk: PublicKey, p256_pk: PublicKey) {
    let old_contract_id: AccountId = "intents.near".parse().unwrap();
    let mainnet = near_workspaces::mainnet()
        .rpc_addr("https://nearrpc.aurora.dev")
        .await
        .unwrap();

    let sandbox = near_workspaces::sandbox().await.unwrap();
    let new_contract = sandbox
        .import_contract(&old_contract_id, &mainnet)
        .with_data()
        .transact()
        .await
        .unwrap();

    new_contract
        .as_account()
        .deploy(&DEFUSE_WASM)
        .await
        .unwrap()
        .into_result()
        .unwrap();

    assert_eq!(
        new_contract
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
            new_contract
                .has_public_key(&public_key.to_implicit_account_id(), &public_key)
                .await
                .unwrap()
        );

        assert!(
            !new_contract
                .has_public_key(new_contract.id(), &public_key)
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

    // Make some changes existing users:
    let (user1, user2) = futures::join!(env.create_user(), env.create_user());

    let mut persistent_tokens = user1.mt_tokens(env.defuse.id(), ..).await.unwrap();

    // Create new users
    let (user3, user4) = futures::try_join!(
        env.create_named_user("first_new_user"),
        env.create_named_user("second_new_user")
    )
    .expect("Failed to create new users");

    // Create new token
    let ft1 = env.create_token().await;

    // Check users
    {
        env.initial_ft_storage_deposit(
            vec![user1.id(), user2.id(), user3.id(), user4.id()],
            vec![&ft1],
        )
        .await;

        let users = [&user1, &user2, &user3, &user4];

        // Additional deposits to new users
        try_join_all(
            users
                .iter()
                .map(|user| env.defuse_ft_deposit_to(&ft1, 10_000, user.id())),
        )
        .await
        .expect("Failed to deposit to users");

        // Interactions between new and old users
        {
            let payloads =
                futures::future::try_join_all(users.iter().combinations(2).map(|accounts| {
                    let sender = accounts[0];
                    let receiver = accounts[1];
                    sender.create_defuse_payload(
                        env.defuse.id(),
                        [Transfer {
                            receiver_id: receiver.id().clone(),
                            tokens: Amounts::new(
                                [(TokenId::Nep141(Nep141TokenId::new(ft1.clone())), 1000)].into(),
                            ),
                            memo: None,
                        }],
                    )
                }))
                .await
                .unwrap();

            env.defuse
                .execute_intents(env.defuse.id(), payloads)
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
        let tokens = user1.mt_tokens(env.defuse.id(), ..).await.unwrap();

        // New token
        persistent_tokens.push(Token {
            token_id: TokenId::Nep141(Nep141TokenId::new(ft1.clone())).to_string(),
            owner_id: None,
        });

        assert_eq!(tokens, persistent_tokens);
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

        let current_fee = env.defuse.fee(env.defuse.id()).await.unwrap();

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

        let current_salt = env.defuse.current_salt(env.defuse.id()).await.unwrap();

        assert_eq!(new_salt, current_salt);
    }
}
