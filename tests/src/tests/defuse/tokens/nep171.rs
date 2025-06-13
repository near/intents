use crate::tests::defuse::SigningStandard;
use crate::tests::defuse::{DefuseSigner, env::Env, intents::ExecuteIntentsExt};
use crate::utils::{mt::MtExt, nft::NftExt};
use arbitrary::{Arbitrary, Unstructured};
use defuse::core::token_id::TokenId as DefuseTokenId;
use defuse::core::token_id::nep171::Nep171TokenId;
use defuse::core::{
    Deadline,
    intents::{DefuseIntents, tokens::NftWithdraw},
};
use near_contract_standards::non_fungible_token::metadata::{
    NFT_METADATA_SPEC, NFTContractMetadata,
};
use near_contract_standards::non_fungible_token::{Token, metadata::TokenMetadata};
use near_sdk::{NearToken, json_types::Base64VecU8};
use randomness::Rng;
use rstest::rstest;
use std::collections::HashMap;
use test_utils::random::{
    Seed, gen_random_bytes, gen_random_string, make_seedable_rng, random_seed,
};

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
            NFTContractMetadata {
                reference: Some("http://abc.com/xyz/".to_string()),
                reference_hash: Some(Base64VecU8(gen_random_bytes(&mut rng, 32..=32))),
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Token nft1".to_string(),
                symbol: "NFT_TKN".to_string(),
                icon: None,
                base_uri: None,
            },
        )
        .await
        .unwrap();

    let nft1_id = gen_random_string(&mut rng, 32..=32);

    // Create the token id, expected inside the verifier contract
    let nft1_mt_token_id = DefuseTokenId::from(
        Nep171TokenId::new(nft_issuer_contract.id().to_owned(), nft1_id.clone()).unwrap(),
    );

    let nft1: Token = env
        .user1
        .nft_mint(
            nft_issuer_contract.id(),
            &nft1_id,
            env.user2.id(),
            &TokenMetadata::default(),
        )
        .await
        .unwrap();

    assert_eq!(nft1.token_id, nft1_id);
    assert_eq!(nft1.owner_id, *env.user2.id());

    let nft2_id = gen_random_string(&mut rng, 32..=32);

    // Create the token id, expected inside the verifier contract
    let nft2_mt_token_id = DefuseTokenId::from(
        Nep171TokenId::new(nft_issuer_contract.id().to_owned(), nft2_id.clone()).unwrap(),
    );

    let nft2: Token = env
        .user1
        .nft_mint(
            nft_issuer_contract.id(),
            &nft2_id,
            env.user3.id(),
            &TokenMetadata::default(),
        )
        .await
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

    // Let's test the MultiTokenEnumeration interface
    {
        // mt_tokens
        {
            let nfts_in_verifier = env.user1.mt_tokens(env.defuse.id(), ..).await.unwrap();
            assert_eq!(nfts_in_verifier.len(), 2);
            let nfts_in_verifier_map = nfts_in_verifier
                .into_iter()
                .map(|v| (v.token_id.clone(), v))
                .collect::<HashMap<_, _>>();
            assert!(nfts_in_verifier_map.contains_key(&nft1_mt_token_id.to_string()));
            assert!(nfts_in_verifier_map.contains_key(&nft2_mt_token_id.to_string()));
        }

        // mt_tokens_for_owner
        {
            // User1
            {
                let nfts_in_verifier = env
                    .user1
                    .mt_tokens_for_owner(env.defuse.id(), env.user1.id(), ..)
                    .await
                    .unwrap();
                assert_eq!(nfts_in_verifier.len(), 1);
                assert_eq!(
                    nfts_in_verifier[0].owner_id.as_ref().unwrap(),
                    env.user1.id()
                );
            }

            // User2
            {
                let nfts_in_verifier = env
                    .user1
                    .mt_tokens_for_owner(env.defuse.id(), env.user2.id(), ..)
                    .await
                    .unwrap();
                assert_eq!(nfts_in_verifier.len(), 0);
            }

            // User3
            {
                let nfts_in_verifier = env
                    .user1
                    .mt_tokens_for_owner(env.defuse.id(), env.user3.id(), ..)
                    .await
                    .unwrap();
                assert_eq!(nfts_in_verifier.len(), 1);
                assert_eq!(
                    nfts_in_verifier[0].owner_id.as_ref().unwrap(),
                    env.user3.id()
                );
            }
        }
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

        let nonce = rng.random();

        env.defuse
            .execute_intents([env.user3.sign_defuse_message(
                SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>()))
                    .unwrap(),
                env.defuse.id(),
                nonce,
                Deadline::timeout(std::time::Duration::from_secs(120)),
                DefuseIntents {
                    intents: [NftWithdraw {
                        token: nft_issuer_contract.id().clone(),
                        receiver_id: env.user1.id().clone(),
                        token_id: nft1_id,
                        memo: None,
                        msg: None,
                        storage_deposit: None,
                        min_gas: None,
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
