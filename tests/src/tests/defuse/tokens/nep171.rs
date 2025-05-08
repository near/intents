use crate::tests::defuse::DefuseSigner;
use crate::tests::defuse::env::Env;
use crate::tests::defuse::intents::ExecuteIntentsExt;
use crate::utils::mt::MtExt;
use crate::utils::nft::NftExt;
use defuse::core::Deadline;
use defuse::core::intents::DefuseIntents;
use defuse::core::intents::tokens::NftWithdraw;
use defuse::core::tokens::TokenId as MtTokenId;
use near_contract_standards::non_fungible_token::Token;
use near_contract_standards::non_fungible_token::metadata::TokenMetadata;
use near_sdk::NearToken;
use near_sdk::json_types::Base64VecU8;
use randomness::Rng;
use rstest::rstest;
use serde_json::json;
use test_utils::random::Seed;
use test_utils::random::gen_random_bytes;
use test_utils::random::gen_random_string;
use test_utils::random::make_seedable_rng;
use test_utils::random::random_seed;

#[tokio::test]
#[rstest]
async fn transfer_nft_to_verifier(random_seed: Seed) {
    let mut rng = make_seedable_rng(random_seed);

    let env = Env::builder().build().await;

    env.transfer_near(env.user1.id(), NearToken::from_near(100))
        .await
        .unwrap()
        .unwrap();

    let nft_issuer_contract = env
        .user1
        .deploy_vanilla_nft_issuer(
            "nft1",
            Some("http://abc.com/xyz/".to_string()),
            Some(Base64VecU8(gen_random_bytes(&mut rng, 32..=32))),
        )
        .await
        .unwrap();

    let nft1_id = gen_random_string(&mut rng, 32..=32);

    // Create the token id, expected inside the verifier contract
    let nft1_mt_token_id = MtTokenId::Nep171(nft_issuer_contract.id().to_owned(), nft1_id.clone());

    let nft1: Token = env
        .user1
        .call(nft_issuer_contract.id(), "nft_mint")
        .args_json(json!({
            "token_id": nft1_id,
            "token_owner_id": env.user2.id().clone(),
            "token_metadata": TokenMetadata::default(),
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await
        .unwrap()
        .json()
        .unwrap();

    assert_eq!(nft1.token_id, nft1_id);
    assert_eq!(nft1.owner_id, *env.user2.id());

    let nft2_id = gen_random_string(&mut rng, 32..=32);

    // Create the token id, expected inside the verifier contract
    let nft2_mt_token_id = MtTokenId::Nep171(nft_issuer_contract.id().to_owned(), nft2_id.clone());

    let nft2: Token = env
        .user1
        .call(nft_issuer_contract.id(), "nft_mint")
        .args_json(json!({
            "token_id": nft2_id,
            "token_owner_id": env.user3.id().clone(),
            "token_metadata": TokenMetadata::default(),
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await
        .unwrap()
        .json()
        .unwrap();

    assert_eq!(nft2.token_id, nft2_id);
    assert_eq!(nft2.owner_id, *env.user3.id());

    {
        {
            assert_eq!(nft1.owner_id, *env.user2.id());
            assert!(
                env.user2
                    .nft_transfer_call(
                        nft_issuer_contract.id(),
                        env.defuse.id(),
                        nft1.token_id.clone(),
                        None,
                        env.user3.id().to_string(),
                    )
                    .await
                    .unwrap()
            );

            let nft1_data = env
                .user2
                .nft_token(nft_issuer_contract.id(), &nft1.token_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(nft1_data.owner_id, *env.defuse.id());
        }

        // After transferring to defuse, the owner is user3, since it's specified in the message
        assert_eq!(
            env.defuse
                .mt_balance_of(env.user2.id(), &nft1_mt_token_id.to_string())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(env.user3.id(), &nft1_mt_token_id.to_string())
                .await
                .unwrap(),
            1
        );
    }

    {
        {
            assert_eq!(nft2.owner_id, *env.user3.id());
            assert!(
                env.user3
                    .nft_transfer_call(
                        nft_issuer_contract.id(),
                        env.defuse.id(),
                        nft2.token_id.clone(),
                        None,
                        env.user1.id().to_string(),
                    )
                    .await
                    .unwrap()
            );

            let nft2_data = env
                .user2
                .nft_token(nft_issuer_contract.id(), &nft2.token_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(nft2_data.owner_id, *env.defuse.id());
        }

        // After transferring to defuse, the owner is user3, since it's specified in the message
        assert_eq!(
            env.defuse
                .mt_balance_of(env.user3.id(), &nft2_mt_token_id.to_string())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(env.user1.id(), &nft2_mt_token_id.to_string())
                .await
                .unwrap(),
            1
        );
    }

    {
        {
            let nft1_data = env
                .user2
                .nft_token(nft_issuer_contract.id(), &nft1.token_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(nft1_data.owner_id, *env.defuse.id());

            assert_eq!(
                env.defuse
                    .mt_balance_of(env.user3.id(), &nft1_mt_token_id.to_string())
                    .await
                    .unwrap(),
                1
            );
        }

        env.defuse
            .execute_intents([env.user3.sign_defuse_message(
                env.defuse.id(),
                rng.random(),
                Deadline::timeout(std::time::Duration::from_secs(120)),
                DefuseIntents {
                    intents: [NftWithdraw {
                        token: nft_issuer_contract.id().clone(),
                        receiver_id: env.user1.id().clone(),
                        token_id: nft1_id,
                        memo: None,
                        msg: None,
                        storage_deposit: None,
                    }
                    .into()]
                    .into(),
                },
            )])
            .await
            .unwrap();

        // User3 doesn't own the NFT on the verifier contract
        assert_eq!(
            env.defuse
                .mt_balance_of(env.user3.id(), &nft1_mt_token_id.to_string())
                .await
                .unwrap(),
            0
        );

        // After withdrawing to user1, now they own the NFT
        {
            let nft1_data = env
                .user2
                .nft_token(nft_issuer_contract.id(), &nft1.token_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(nft1_data.owner_id, *env.user1.id());
        }
    }
}
