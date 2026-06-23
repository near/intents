use defuse_sandbox::{
    account::Account,
    extensions::{
        DEFAULT_GAS,
        defuse::{
            DefuseExt, DefuseSignerExt,
            core::{
                amounts::Amounts,
                intents::tokens::{NftWithdraw, NotifyOnTransfer, Transfer},
                token_id::{TokenId as DefuseTokenId, nep171::Nep171TokenId},
            },
            tokens::{DepositAction, DepositMessage, ExecuteIntents},
        },
        mt::{Mt, MtBalanceOfArgs, MtExt},
        nft::NftAdminExt,
    },
    kit::Final,
};
use defuse_test_utils::wasms::{MT_RECEIVER_STUB_WASM, NON_FUNGIBLE_TOKEN_WASM};
use multi_token_receiver_stub::MTReceiverMode as StubAction;
use near_contract_standards::non_fungible_token::{
    Token,
    metadata::{NFT_METADATA_SPEC, NFTContractMetadata, TokenMetadata},
};
use near_sdk::{NearToken, json_types::Base64VecU8};
use rstest::rstest;
use std::collections::HashMap;

use crate::tests::defuse::env::Env;

const DUMMY_REFERENCE_HASH: [u8; 32] = [33; 32];
const DUMMY_NFT1_ID: &str = "thisisdummynftid1";
const DUMMY_NFT2_ID: &str = "thisisdummythisisdummynnthisisdummynftid2";

#[rstest]
#[tokio::test]
async fn transfer_nft_to_verifier() {
    let env = Env::builder().build().await;

    let (user1, user2, user3) = futures::join!(
        env.create_named_user("nft_issuer_admin"),
        env.create_user(),
        env.create_user()
    );

    env.transaction(user1.account_id())
        .transfer(NearToken::from_near(100))
        .await
        .unwrap();

    let existing_tokens = env.mt_tokens(env.defuse.contract_id(), ..).await.unwrap();

    let nft_issuer_contract = user1
        .deploy_vanilla_nft_issuer(
            "nft1",
            user1.account_id(),
            NFTContractMetadata {
                reference: Some("http://abc.com/xyz/".to_string()),
                reference_hash: Some(Base64VecU8(DUMMY_REFERENCE_HASH.to_vec())),
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Token nft1".to_string(),
                symbol: "NFT_TKN".to_string(),
                icon: None,
                base_uri: None,
            },
            NON_FUNGIBLE_TOKEN_WASM.clone(),
        )
        .await;

    // Create the token id, expected inside the verifier contract
    let nft1_mt_token_id = DefuseTokenId::from(Nep171TokenId::new(
        nft_issuer_contract.contract_id().to_owned(),
        DUMMY_NFT1_ID.to_string(),
    ));

    let nft1: Token = user1
        .mint_nft(
            nft_issuer_contract.contract_id(),
            &DUMMY_NFT1_ID.to_string(),
            user2.account_id(),
            &TokenMetadata::default(),
        )
        .await
        .unwrap();

    assert_eq!(nft1.token_id, DUMMY_NFT1_ID.to_string());
    assert_eq!(nft1.owner_id, *user2.account_id());

    // Create the token id, expected inside the verifier contract
    let nft2_mt_token_id = DefuseTokenId::from(Nep171TokenId::new(
        nft_issuer_contract.contract_id().to_owned(),
        DUMMY_NFT2_ID.to_string(),
    ));

    let nft2: Token = user1
        .mint_nft(
            nft_issuer_contract.contract_id(),
            &DUMMY_NFT2_ID.to_string(),
            user3.account_id(),
            &TokenMetadata::default(),
        )
        .await
        .unwrap();

    assert_eq!(nft2.token_id, DUMMY_NFT2_ID.to_string());
    assert_eq!(nft2.owner_id, *user3.account_id());

    {
        {
            assert_eq!(nft1.owner_id, *user2.account_id());

            user2
                .nft(nft_issuer_contract.contract_id())
                .unwrap()
                .transfer_call(
                    env.defuse.contract_id(),
                    nft1.token_id.clone(),
                    near_sdk::serde_json::to_string(&DepositMessage::new(
                        user3.account_id().clone(),
                    ))
                    .unwrap(),
                )
                .gas(DEFAULT_GAS)
                .wait_until(Final)
                .await
                .unwrap();

            let nft1_data = nft_issuer_contract
                .token(&nft1.token_id)
                .await
                .unwrap()
                .unwrap();

            assert_eq!(nft1_data.owner_id, *env.defuse.contract_id());
        }

        // After transferring to defuse, the owner is user3, since it's specified in the message
        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: user2.account_id(),
                    token_id: &nft1_mt_token_id.to_string(),
                })
                .await
                .unwrap()
                .0,
            0
        );

        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: user3.account_id(),
                    token_id: &nft1_mt_token_id.to_string(),
                })
                .await
                .unwrap()
                .0,
            1
        );
    }

    {
        {
            assert_eq!(nft2.owner_id, *user3.account_id());
            user3
                .nft(nft_issuer_contract.contract_id())
                .unwrap()
                .transfer_call(
                    env.defuse.contract_id(),
                    nft2.token_id.clone(),
                    near_sdk::serde_json::to_string(&DepositMessage::new(
                        user1.account_id().clone(),
                    ))
                    .unwrap(),
                )
                .gas(DEFAULT_GAS)
                .wait_until(Final)
                .await
                .unwrap();

            let nft2_data = nft_issuer_contract
                .token(&nft2.token_id)
                .await
                .unwrap()
                .unwrap();

            assert_eq!(nft2_data.owner_id, *env.defuse.contract_id());
        }

        // After transferring to defuse, the owner is user3, since it's specified in the message
        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: user3.account_id(),
                    token_id: &nft2_mt_token_id.to_string(),
                })
                .await
                .unwrap()
                .0,
            0
        );
        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: user1.account_id(),
                    token_id: &nft2_mt_token_id.to_string(),
                })
                .await
                .unwrap()
                .0,
            1
        );
    }

    // Let's test the MultiTokenEnumeration interface
    {
        // mt_tokens
        {
            let nfts_in_verifier = env.mt_tokens(env.defuse.contract_id(), ..).await.unwrap();

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
                let nfts_in_verifier = env
                    .mt_tokens_for_owner(env.defuse.contract_id(), user1.account_id(), ..)
                    .await
                    .unwrap();
                assert_eq!(nfts_in_verifier.len(), 1);
                assert_eq!(
                    nfts_in_verifier[0].owner_id.as_ref().unwrap(),
                    user1.account_id()
                );
            }

            // User2
            {
                let nfts_in_verifier = env
                    .mt_tokens_for_owner(env.defuse.contract_id(), user2.account_id(), ..)
                    .await
                    .unwrap();
                assert_eq!(nfts_in_verifier.len(), 0);
            }

            // User3
            {
                let nfts_in_verifier = env
                    .mt_tokens_for_owner(env.defuse.contract_id(), user3.account_id(), ..)
                    .await
                    .unwrap();
                assert_eq!(nfts_in_verifier.len(), 1);
                assert_eq!(
                    nfts_in_verifier[0].owner_id.as_ref().unwrap(),
                    user3.account_id()
                );
            }
        }
    }

    {
        {
            let nft1_data = nft_issuer_contract
                .token(&nft1.token_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(nft1_data.owner_id, *env.defuse.contract_id());

            assert_eq!(
                env.contract::<Mt>(env.defuse.contract_id())
                    .mt_balance_of(MtBalanceOfArgs {
                        account_id: user3.account_id(),
                        token_id: &nft1_mt_token_id.to_string(),
                    })
                    .await
                    .unwrap()
                    .0,
                1
            );
        }

        let withdraw_payload = user3
            .sign_defuse_payload_default(
                &env.defuse,
                [NftWithdraw {
                    token: nft_issuer_contract.contract_id().clone(),
                    receiver_id: user1.account_id().clone(),
                    token_id: DUMMY_NFT1_ID.to_string(),
                    memo: None,
                    msg: None,
                    storage_deposit: None,
                    min_gas: None,
                }],
            )
            .await
            .unwrap();

        env.defuse_simulate_and_execute_intents(env.defuse.contract_id(), [withdraw_payload])
            .await
            .unwrap();

        // User3 doesn't own the NFT on the verifier contract
        assert_eq!(
            env.contract::<Mt>(env.defuse.contract_id())
                .mt_balance_of(MtBalanceOfArgs {
                    account_id: user3.account_id(),
                    token_id: &nft1_mt_token_id.to_string(),
                })
                .await
                .unwrap()
                .0,
            0
        );

        // After withdrawing to user1, now they own the NFT
        {
            let nft1_data = nft_issuer_contract
                .token(&nft1.token_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(nft1_data.owner_id, *user1.account_id());
        }
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
struct NftTransferCallExpectation {
    action: StubAction,
    intent_transfer: bool,
    refund_if_fails: bool,
    expected_sender_owns_nft: bool,
    expected_receiver_owns_nft: bool,
}

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
    expected_sender_owns_nft: true,
    expected_receiver_owns_nft: false,
})]
#[case::malicious_receiver(NftTransferCallExpectation {
    action: StubAction::MaliciousReturn,
    intent_transfer: false,
    refund_if_fails: true,
    expected_sender_owns_nft: true,
    expected_receiver_owns_nft: false,
})]
#[tokio::test]
async fn nft_transfer_call_calls_mt_on_transfer_variants(
    #[case] expectation: NftTransferCallExpectation,
) {
    let env = Env::builder().deployer_as_super_admin().build().await;

    // Ensure the NFT issuer account name stays short enough to host `nft_test.<user>`
    // subaccounts; randomly generated names occasionally exceed the NEAR 64-char limit.
    let (user, intent_receiver) = futures::join!(
        env.create_named_user("nft_transfer_sender"),
        env.create_user()
    );

    let receiver = env
        .deploy_sub_contract(
            "receiver_stub",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None,
        )
        .await
        .unwrap();

    env.transaction(user.account_id())
        .transfer(NearToken::from_near(100))
        .await
        .unwrap()
        .result()
        .unwrap();

    let nft_issuer_contract = user
        .deploy_vanilla_nft_issuer(
            "nft_test",
            user.account_id(),
            NFTContractMetadata {
                reference: Some("http://test.com/".to_string()),
                reference_hash: Some(Base64VecU8(DUMMY_REFERENCE_HASH.to_vec())),
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Test NFT".to_string(),
                symbol: "TNFT".to_string(),
                icon: None,
                base_uri: None,
            },
            NON_FUNGIBLE_TOKEN_WASM.clone(),
        )
        .await;

    let nft = user
        .mint_nft(
            nft_issuer_contract.contract_id(),
            &DUMMY_NFT1_ID.to_string(),
            user.account_id(),
            &TokenMetadata::default(),
        )
        .await
        .unwrap();

    assert_eq!(nft.owner_id, *user.account_id());

    let nft_token_id = DefuseTokenId::from(Nep171TokenId::new(
        nft_issuer_contract.contract_id().clone(),
        DUMMY_NFT1_ID.to_string(),
    ));

    let intents = if expectation.intent_transfer {
        vec![
            receiver
                .sign_defuse_payload_default(
                    &env.defuse,
                    [Transfer {
                        receiver_id: intent_receiver.account_id().clone(),
                        tokens: Amounts::new(std::iter::once((nft_token_id.clone(), 1)).collect()),
                        memo: None,
                        notification: None,
                    }],
                )
                .await
                .unwrap(),
        ]
    } else {
        vec![]
    };

    let deposit_message = if intents.is_empty() {
        DepositMessage {
            receiver_id: receiver.account_id().clone(),
            action: Some(DepositAction::Notify(NotifyOnTransfer::new(
                near_sdk::serde_json::to_string(&expectation.action).unwrap(),
            ))),
        }
    } else {
        DepositMessage {
            receiver_id: receiver.account_id().clone(),
            action: Some(DepositAction::Execute(ExecuteIntents {
                execute_intents: intents,
                refund_if_fails: expectation.refund_if_fails,
            })),
        }
    };

    user.nft(nft_issuer_contract.contract_id())
        .unwrap()
        .transfer_call(
            env.defuse.contract_id(),
            nft.token_id.clone(),
            near_sdk::serde_json::to_string(&deposit_message).unwrap(),
        )
        .gas(DEFAULT_GAS)
        .wait_until(Final)
        .await
        .unwrap();

    // Check ownership on the NFT contract
    let nft_owner = nft_issuer_contract
        .token(&nft.token_id)
        .await
        .unwrap()
        .unwrap()
        .owner_id;

    if expectation.expected_sender_owns_nft {
        assert_eq!(
            nft_owner,
            *user.account_id(),
            "NFT should be owned by sender"
        );
    } else {
        assert_eq!(
            nft_owner,
            *env.defuse.contract_id(),
            "NFT should be owned by defuse contract"
        );
    }

    // Check if receiver owns the NFT in MT balance
    let receiver_mt_balance = env
        .contract::<Mt>(env.defuse.contract_id())
        .mt_balance_of(MtBalanceOfArgs {
            account_id: receiver.account_id(),
            token_id: &nft_token_id.to_string(),
        })
        .await
        .unwrap();

    if expectation.expected_receiver_owns_nft {
        assert_eq!(
            receiver_mt_balance.0, 1,
            "Receiver should own the NFT (MT balance = 1)"
        );
    } else {
        assert_eq!(
            receiver_mt_balance.0, 0,
            "Receiver should not own the NFT (MT balance = 0)"
        );
    }
}
