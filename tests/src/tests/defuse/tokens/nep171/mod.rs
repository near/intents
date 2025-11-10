use crate::tests::defuse::DefuseSignerExt;
use crate::tests::defuse::{env::Env, intents::ExecuteIntentsExt};
use crate::utils::{mt::MtExt, nft::NftExt};
use defuse::core::intents::tokens::NftWithdraw;
use defuse::core::token_id::TokenId as DefuseTokenId;
use defuse::core::token_id::nep171::Nep171TokenId;
use near_contract_standards::non_fungible_token::metadata::{
    NFT_METADATA_SPEC, NFTContractMetadata,
};
use near_contract_standards::non_fungible_token::{Token, metadata::TokenMetadata};
use near_sdk::{NearToken, json_types::Base64VecU8};
use rstest::rstest;
use multi_token_receiver_stub::StubAction;
use std::collections::HashMap;

const DUMMY_REFERENCE_HASH: [u8; 32] = [33; 32];
const DUMMY_NFT1_ID: &str = "thisisdummynftid1";
const DUMMY_NFT2_ID: &str = "thisisdummythisisdummynnthisisdummynftid2";

#[tokio::test]
#[rstest]
async fn transfer_nft_to_verifier() {
    let env = Env::builder().create_unique_users().build().await;

    let (user1, user2, user3) = futures::join!(
        env.create_named_user("nft_issuer_admin"),
        env.create_user(),
        env.create_user()
    );

    env.transfer_near(user1.id(), NearToken::from_near(100))
        .await
        .unwrap()
        .unwrap();

    let existing_tokens = user1.mt_tokens(env.defuse.id(), ..).await.unwrap();

    let nft_issuer_contract = user1
        .deploy_vanilla_nft_issuer(
            "nft1",
            NFTContractMetadata {
                reference: Some("http://abc.com/xyz/".to_string()),
                reference_hash: Some(Base64VecU8(DUMMY_REFERENCE_HASH.to_vec())),
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Token nft1".to_string(),
                symbol: "NFT_TKN".to_string(),
                icon: None,
                base_uri: None,
            },
        )
        .await
        .unwrap();

    // Create the token id, expected inside the verifier contract
    let nft1_mt_token_id = DefuseTokenId::from(
        Nep171TokenId::new(
            nft_issuer_contract.id().to_owned(),
            DUMMY_NFT1_ID.to_string(),
        )
        .unwrap(),
    );

    let nft1: Token = user1
        .nft_mint(
            nft_issuer_contract.id(),
            &DUMMY_NFT1_ID.to_string(),
            user2.id(),
            &TokenMetadata::default(),
        )
        .await
        .unwrap();

    assert_eq!(nft1.token_id, DUMMY_NFT1_ID.to_string());
    assert_eq!(nft1.owner_id, *user2.id());

    // Create the token id, expected inside the verifier contract
    let nft2_mt_token_id = DefuseTokenId::from(
        Nep171TokenId::new(
            nft_issuer_contract.id().to_owned(),
            DUMMY_NFT2_ID.to_string(),
        )
        .unwrap(),
    );

    let nft2: Token = user1
        .nft_mint(
            nft_issuer_contract.id(),
            &DUMMY_NFT2_ID.to_string(),
            user3.id(),
            &TokenMetadata::default(),
        )
        .await
        .unwrap();

    assert_eq!(nft2.token_id, DUMMY_NFT2_ID.to_string());
    assert_eq!(nft2.owner_id, *user3.id());

    {
        {
            assert_eq!(nft1.owner_id, *user2.id());
            assert!(
                user2
                    .nft_transfer_call(
                        nft_issuer_contract.id(),
                        env.defuse.id(),
                        nft1.token_id.clone(),
                        None,
                        user3.id().to_string(),
                    )
                    .await
                    .unwrap()
            );

            let nft1_data = user2
                .nft_token(nft_issuer_contract.id(), &nft1.token_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(nft1_data.owner_id, *env.defuse.id());
        }

        // After transferring to defuse, the owner is user3, since it's specified in the message
        assert_eq!(
            env.defuse
                .mt_balance_of(user2.id(), &nft1_mt_token_id.to_string())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(user3.id(), &nft1_mt_token_id.to_string())
                .await
                .unwrap(),
            1
        );
    }

    {
        {
            assert_eq!(nft2.owner_id, *user3.id());
            assert!(
                user3
                    .nft_transfer_call(
                        nft_issuer_contract.id(),
                        env.defuse.id(),
                        nft2.token_id.clone(),
                        None,
                        user1.id().to_string(),
                    )
                    .await
                    .unwrap()
            );

            let nft2_data = user2
                .nft_token(nft_issuer_contract.id(), &nft2.token_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(nft2_data.owner_id, *env.defuse.id());
        }

        // After transferring to defuse, the owner is user3, since it's specified in the message
        assert_eq!(
            env.defuse
                .mt_balance_of(user3.id(), &nft2_mt_token_id.to_string())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            env.defuse
                .mt_balance_of(user1.id(), &nft2_mt_token_id.to_string())
                .await
                .unwrap(),
            1
        );
    }

    // Let's test the MultiTokenEnumeration interface
    {
        // mt_tokens
        {
            let nfts_in_verifier = user1.mt_tokens(env.defuse.id(), ..).await.unwrap();

            assert_eq!(nfts_in_verifier.len(), existing_tokens.len() + 2);

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
                let nfts_in_verifier = user1
                    .mt_tokens_for_owner(env.defuse.id(), user1.id(), ..)
                    .await
                    .unwrap();
                assert_eq!(nfts_in_verifier.len(), 1);
                assert_eq!(nfts_in_verifier[0].owner_id.as_ref().unwrap(), user1.id());
            }

            // User2
            {
                let nfts_in_verifier = user1
                    .mt_tokens_for_owner(env.defuse.id(), user2.id(), ..)
                    .await
                    .unwrap();
                assert_eq!(nfts_in_verifier.len(), 0);
            }

            // User3
            {
                let nfts_in_verifier = user1
                    .mt_tokens_for_owner(env.defuse.id(), user3.id(), ..)
                    .await
                    .unwrap();
                assert_eq!(nfts_in_verifier.len(), 1);
                assert_eq!(nfts_in_verifier[0].owner_id.as_ref().unwrap(), user3.id());
            }
        }
    }

    {
        {
            let nft1_data = user2
                .nft_token(nft_issuer_contract.id(), &nft1.token_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(nft1_data.owner_id, *env.defuse.id());

            assert_eq!(
                env.defuse
                    .mt_balance_of(user3.id(), &nft1_mt_token_id.to_string())
                    .await
                    .unwrap(),
                1
            );
        }

        let withdraw_payload = user3
            .sign_defuse_payload_default(
                env.defuse.id(),
                [NftWithdraw {
                    token: nft_issuer_contract.id().clone(),
                    receiver_id: user1.id().clone(),
                    token_id: DUMMY_NFT1_ID.to_string(),
                    memo: None,
                    msg: None,
                    storage_deposit: None,
                    min_gas: None,
                }],
            )
            .await
            .unwrap();

        env.defuse
            .execute_intents(env.defuse.id(), [withdraw_payload])
            .await
            .unwrap();

        // User3 doesn't own the NFT on the verifier contract
        assert_eq!(
            env.defuse
                .mt_balance_of(user3.id(), &nft1_mt_token_id.to_string())
                .await
                .unwrap(),
            0
        );

        // After withdrawing to user1, now they own the NFT
        {
            let nft1_data = user2
                .nft_token(nft_issuer_contract.id(), &nft1.token_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(nft1_data.owner_id, *user1.id());
        }
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
struct NftTransferCallExpectation {
    action: multi_token_receiver_stub::StubAction,
    intent_transfer: bool,
    refund_if_fails: bool,
    expected_sender_owns_nft: bool,
    expected_receiver_owns_nft: bool,
}

#[tokio::test]
#[rstest]
#[case::nothing_to_refund(NftTransferCallExpectation {
    action: StubAction::ReturnValue(0.into()),
    intent_transfer: false,
    refund_if_fails: true,
    expected_sender_owns_nft: false,
    expected_receiver_owns_nft: true,
})]
#[case::request_refund(NftTransferCallExpectation {
    action: StubAction::ReturnValue(1.into()),
    intent_transfer: false,
    refund_if_fails: true,
    expected_sender_owns_nft: true,
    expected_receiver_owns_nft: false,
})]
#[case::receiver_panics(NftTransferCallExpectation {
    action: StubAction::Panic,
    intent_transfer: false,
    refund_if_fails: true,
    expected_sender_owns_nft: false,
    expected_receiver_owns_nft: true,
})]
#[case::malicious_receiver(NftTransferCallExpectation {
    action: StubAction::MaliciousReturn,
    intent_transfer: false,
    refund_if_fails: true,
    expected_sender_owns_nft: false,
    expected_receiver_owns_nft: true,
})]
#[case::cannot_refund_after_nft_transfer_to_another_user_thorough_intent(NftTransferCallExpectation {
    action: StubAction::ReturnValue(1.into()),
    intent_transfer: true,
    refund_if_fails: true,
    expected_sender_owns_nft: false,
    expected_receiver_owns_nft: false,
})]
#[case::no_refund_after_transfer_intent_kept(NftTransferCallExpectation {
    action: StubAction::ReturnValue(0.into()),
    intent_transfer: true,
    refund_if_fails: false,
    expected_sender_owns_nft: false,
    expected_receiver_owns_nft: false,
})]
async fn nft_transfer_call_calls_mt_on_transfer_variants(
    #[case] expectation: NftTransferCallExpectation,
) {
    use defuse::core::{amounts::Amounts, intents::tokens::Transfer};
    use defuse::tokens::DepositMessage;
    use crate::tests::defuse::env::MT_RECEIVER_STUB_WASM;

    let env = Env::builder()
        .deployer_as_super_admin()
        .build()
        .await;

    // Ensure the NFT issuer account name stays short enough to host `nft_test.<user>`
    // subaccounts; randomly generated names occasionally exceed the NEAR 64-char limit.
    let (user, receiver, intent_receiver) = futures::join!(
        env.create_named_user("nft_transfer_sender"),
        env.create_user(),
        env.create_user()
    );

    receiver
        .deploy(MT_RECEIVER_STUB_WASM.as_slice())
        .await
        .unwrap()
        .unwrap();

    env.transfer_near(user.id(), NearToken::from_near(100))
        .await
        .unwrap()
        .unwrap();

    let nft_issuer_contract = user
        .deploy_vanilla_nft_issuer(
            "nft_test",
            NFTContractMetadata {
                reference: Some("http://test.com/".to_string()),
                reference_hash: Some(Base64VecU8(DUMMY_REFERENCE_HASH.to_vec())),
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Test NFT".to_string(),
                symbol: "TNFT".to_string(),
                icon: None,
                base_uri: None,
            },
        )
        .await
        .unwrap();

    let nft: Token = user
        .nft_mint(
            nft_issuer_contract.id(),
            &DUMMY_NFT1_ID.to_string(),
            user.id(),
            &TokenMetadata::default(),
        )
        .await
        .unwrap();

    assert_eq!(nft.token_id, DUMMY_NFT1_ID.to_string());
    assert_eq!(nft.owner_id, *user.id());

    let nft_token_id = DefuseTokenId::from(
        Nep171TokenId::new(
            nft_issuer_contract.id().clone(),
            DUMMY_NFT1_ID.to_string(),
        )
        .unwrap(),
    );

    let intents = if expectation.intent_transfer {
        vec![receiver
            .sign_defuse_payload_default(
                env.defuse.id(),
                [Transfer {
                    receiver_id: intent_receiver.id().clone(),
                    tokens: Amounts::new(std::iter::once((nft_token_id.clone(), 1)).collect()),
                    memo: None,
                }],
            )
            .await
            .unwrap()]
    } else {
        vec![]
    };

    let deposit_message = DepositMessage {
        receiver_id: receiver.id().clone(),
        execute_intents: intents,
        refund_if_fails: expectation.refund_if_fails,
        message: near_sdk::serde_json::to_string(&expectation.action).unwrap(),
    };

    user.nft_transfer_call(
        nft_issuer_contract.id(),
        env.defuse.id(),
        nft.token_id.clone(),
        None,
        near_sdk::serde_json::to_string(&deposit_message).unwrap(),
    )
    .await
    .unwrap();

    // Check ownership on the NFT contract
    let nft_owner = user
        .nft_token(nft_issuer_contract.id(), &nft.token_id)
        .await
        .unwrap()
        .unwrap()
        .owner_id;

    if expectation.expected_sender_owns_nft {
        assert_eq!(nft_owner, *user.id(), "NFT should be owned by sender");
    } else {
        assert_eq!(nft_owner, *env.defuse.id(), "NFT should be owned by defuse contract");
    }

    // Check if receiver owns the NFT in MT balance
    let receiver_mt_balance = env
        .mt_contract_balance_of(env.defuse.id(), receiver.id(), &nft_token_id.to_string())
        .await
        .unwrap();

    if expectation.expected_receiver_owns_nft {
        assert_eq!(receiver_mt_balance, 1, "Receiver should own the NFT (MT balance = 1)");
    } else {
        assert_eq!(receiver_mt_balance, 0, "Receiver should not own the NFT (MT balance = 0)");
    }
}
