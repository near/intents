use crate::tests::defuse::env::Env;
use crate::utils::mt::MtExt;
use crate::utils::nft::NftExt;
use defuse::core::tokens::TokenId as MtTokenId;
use near_contract_standards::non_fungible_token::Token;
use near_contract_standards::non_fungible_token::metadata::TokenMetadata;
use near_sdk::NearToken;
use near_sdk::json_types::Base64VecU8;
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

    let nft1_mt_token_id =
        MtTokenId::Nep171(nft_issuer_contract.id().to_owned(), nft1.token_id.clone());

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
