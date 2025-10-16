use crate::{tests::defuse::{DefuseExt, DefuseSigner}, tests::defuse::accounts::AccountManagerExt, tests::defuse::env::Env};
use crate::tests::defuse::intents::ExecuteIntentsExt;
use crate::tests::defuse::SigningStandard;
use crate::tests::utils::AsNearSdkLog;
use crate::utils::{crypto::Signer, ft::FtExt, mt::MtExt, nft::NftExt, test_log::TestLog, wnear::WNearExt};
use defuse_crypto::{Payload, PublicKey};
use arbitrary::{Arbitrary, Unstructured};
use defuse::contract::config::{DefuseConfig, RolesConfig};
use defuse::core::fees::{FeesConfig, Pips};
use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;
use defuse::core::token_id::nep171::Nep171TokenId;
use defuse::{
    core::{
        Deadline,
        accounts::AccountEvent,
        amounts::Amounts,
        events::DefuseEvent,
        intents::{
            DefuseIntents, IntentEvent,
            account::{AddPublicKey, RemovePublicKey},
            token_diff::{TokenDiff, TokenDeltas},
            tokens::{FtWithdraw, MtWithdraw, NftWithdraw, StorageDeposit, Transfer},
        },
        payload::{DefusePayload, ExtractDefusePayload, multi::MultiPayload},
    },
    intents::SimulationOutput,
};
use defuse_randomness::Rng;
use defuse_test_utils::random::{gen_random_string, random_bytes, rng};
use near_contract_standards::non_fungible_token::metadata::{
    NFT_METADATA_SPEC, NFTContractMetadata, TokenMetadata,
};
use near_sdk::{AccountId, AccountIdRef, NearToken, json_types::Base64VecU8};
use rstest::rstest;
use serde_json::json;
use std::borrow::Cow;


#[tokio::test]
#[rstest]
#[trace]
async fn simulate_ft_transfer_intent(
    #[notrace] mut rng: impl Rng,
) {
    let env = Env::builder()
        .no_registration(true)
        .build()
        .await;

    let ft1 = TokenId::from(Nep141TokenId::new(env.ft1.clone()));

    // deposit
    env.defuse_ft_deposit_to(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();

    let nonce = rng.random();

    let transfer_intent = Transfer {
        receiver_id: env.user2.id().clone(),
        tokens: Amounts::new(std::iter::once((ft1.clone(), 1000)).collect()),
        memo: None,
    };
    let transfer_intent_payload = env.user1.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![transfer_intent.clone().into()],
        },
    );
    let result = env
        .defuse
        .simulate_intents([transfer_intent_payload.clone()])
        .await
        .unwrap();

    assert_eq!(result.intents_executed.len(), 1);

    // Prepare expected transfer event
    let expected_log = DefuseEvent::Transfer(Cow::Owned(vec![IntentEvent {
            intent_hash: transfer_intent_payload.hash(),
            event: AccountEvent {
                account_id: env.user1.id().clone().into(),
                event: Cow::Owned(transfer_intent),
            },
        }]))
        .as_near_sdk_log();

    result.logs.iter().for_each(|log| println!("{}", log));
    println!("{}", near_sdk::serde_json::to_string_pretty(&result).unwrap());

    assert!(result.logs.iter().any(|log| log == &expected_log));
    //TODO: update
    // assert_eq!(
    //     result.intents_executed.first().unwrap().event.event.nonce,
    //     nonce
    // );
    result.into_result().unwrap();

}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_transfer_intent(
    #[notrace] mut rng: impl Rng,
) {
    let env = Env::builder()
        .no_registration(true)
        .build()
        .await;

    let ft1 = TokenId::from(Nep141TokenId::new(env.ft1.clone()));

    // deposit
    env.defuse_ft_deposit_to(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();

    let nonce = rng.random();

    let transfer_intent = Transfer {
        receiver_id: env.user2.id().clone(),
        tokens: Amounts::new(std::iter::once((ft1.clone(), 1000)).collect()),
        memo: None,
    };
    let transfer_intent_payload = env.user1.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![transfer_intent.clone().into()],
        },
    );
    let result = env
        .defuse
        .simulate_intents([transfer_intent_payload.clone()])
        .await
        .unwrap();

    assert_eq!(result.intents_executed.len(), 1);

    // Prepare expected transfer event
    let expected_log = DefuseEvent::Transfer(Cow::Owned(vec![IntentEvent {
            intent_hash: transfer_intent_payload.hash(),
            event: AccountEvent {
                account_id: env.user1.id().clone().into(),
                event: Cow::Owned(transfer_intent),
            },
        }]))
        .as_near_sdk_log();

    result.logs.iter().for_each(|log| println!("{}", log));
    println!("{}", near_sdk::serde_json::to_string_pretty(&result).unwrap());

    assert!(result.logs.iter().any(|log| log == &expected_log));
    //TODO: update
    // assert_eq!(
    //     result.intents_executed.first().unwrap().event.event.nonce,
    //     nonce
    // );
    result.into_result().unwrap()

}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_nft_withdraw_intent(
    #[notrace] mut rng: impl Rng,
) {
    let env = Env::builder()
        .no_registration(true)
        .build()
        .await;

    // Transfer NEAR to user1 for NFT contract deployment
    env.transfer_near(env.user1.id(), NearToken::from_near(100))
        .await
        .unwrap()
        .unwrap();

    // Deploy NFT contract
    let nft_contract = env
        .user1
        .deploy_vanilla_nft_issuer(
            "nft1",
            NFTContractMetadata {
                reference: Some("http://example.com/nft/".to_string()),
                reference_hash: Some(Base64VecU8(random_bytes(32..=32, &mut rng))),
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Test NFT".to_string(),
                symbol: "TNFT".to_string(),
                icon: None,
                base_uri: None,
            },
        )
        .await
        .unwrap();

    // Mint NFT to user1
    let nft_id = gen_random_string(&mut rng, 32..=32);
    let nft = env
        .user1
        .nft_mint(
            nft_contract.id(),
            &nft_id,
            env.user1.id(),
            &TokenMetadata::default(),
        )
        .await
        .unwrap();

    // Create the token id for the NFT in Defuse
    let nft_token_id = TokenId::from(
        Nep171TokenId::new(nft_contract.id().to_owned(), nft_id.clone()).unwrap(),
    );

    // Transfer NFT to Defuse contract (deposit)
    env.user1
        .nft_transfer_call(
            nft_contract.id(),
            env.defuse.id(),
            nft_id.clone(),
            None,
            env.user1.id().to_string(), // Recipient in Defuse
        )
        .await
        .unwrap();

    // Verify NFT is deposited in Defuse
    assert_eq!(
        env.defuse
            .mt_balance_of(env.user1.id(), &nft_token_id.to_string())
            .await
            .unwrap(),
        1
    );

    let nonce = rng.random();

    // Create NftWithdraw intent
    let nft_withdraw_intent = NftWithdraw {
        token: nft_contract.id().clone(),
        receiver_id: env.user2.id().clone(),
        token_id: nft_id.clone(),
        memo: None,
        msg: None,
        storage_deposit: None,
        min_gas: None,
    };

    let nft_withdraw_payload = env.user1.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![nft_withdraw_intent.clone().into()],
        },
    );

    // Simulate the intent
    let result = env
        .defuse
        .simulate_intents([nft_withdraw_payload.clone()])
        .await
        .unwrap();

    assert_eq!(result.intents_executed.len(), 1);

    // Prepare expected NftWithdraw event
    let expected_log = DefuseEvent::NftWithdraw(Cow::Owned(vec![IntentEvent {
            intent_hash: nft_withdraw_payload.hash(),
            event: AccountEvent {
                account_id: env.user1.id().clone().into(),
                event: Cow::Owned(nft_withdraw_intent),
            },
        }]))
        .as_near_sdk_log();

    result.logs.iter().for_each(|log| println!("{}", log));
    println!("{}", near_sdk::serde_json::to_string_pretty(&result).unwrap());

    assert!(result.logs.iter().any(|log| log == &expected_log));
    result.into_result().unwrap();
}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_mt_withdraw_intent(
    #[notrace] mut rng: impl Rng,
) {
    let env = Env::builder()
        .build()
        .await;

    // Deploy a second Defuse contract which supports NEP-245 operations
    let defuse2 = env
        .deploy_defuse(
            "defuse2",
            DefuseConfig {
                wnear_id: env.wnear.id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: env.id().clone(),
                },
                roles: RolesConfig::default(),
            },
        )
        .await
        .unwrap();

    // Register user1's public key on defuse2
    let user1_secret_key: near_crypto::SecretKey = env.user1.secret_key().to_string().parse().unwrap();
    if let near_crypto::PublicKey::ED25519(pk) = user1_secret_key.public_key() {
        env.user1
            .add_public_key(defuse2.id(), defuse_crypto::PublicKey::Ed25519(pk.0))
            .await
            .unwrap();
    }

    let ft1 = TokenId::from(Nep141TokenId::new(env.ft1.clone()));

    // Step 1: Deposit FT to user1 in the first Defuse contract (stored as MT internally)
    env.defuse_ft_deposit_to(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();

    // Verify balance in first Defuse contract
    assert_eq!(
        env.defuse
            .mt_balance_of(env.user1.id(), &ft1.to_string())
            .await
            .unwrap(),
        1000
    );

    // Step 2: Transfer tokens from defuse1 to defuse2 using mt_transfer_call
    // This creates MT tokens in defuse2 that can be withdrawn
    env.user1
        .mt_transfer_call(
            env.defuse.id(),
            defuse2.id(),
            &ft1.to_string(),
            500,
            None,
            None,
            env.user1.id().to_string(), // user1 will own these tokens in defuse2
        )
        .await
        .unwrap();

    // Verify tokens are now in defuse2 as NEP-245 tokens
    use defuse::core::token_id::nep245::Nep245TokenId;
    let nep245_token_id = TokenId::from(
        Nep245TokenId::new(env.defuse.id().to_owned(), ft1.to_string()).unwrap()
    );

    assert_eq!(
        defuse2
            .mt_balance_of(env.user1.id(), &nep245_token_id.to_string())
            .await
            .unwrap(),
        500
    );

    let nonce = rng.random();

    // Step 3: Create MtWithdraw intent to withdraw MT tokens from defuse2 back to defuse1
    // Now we're simulating on defuse2, withdrawing to defuse1
    let mt_withdraw_intent = MtWithdraw {
        token: env.defuse.id().clone(), // External NEP-245 contract (defuse1)
        receiver_id: env.user2.id().clone(), // Withdraw to user2's account in defuse1
        token_ids: vec![ft1.to_string()], // The FT token ID within defuse1
        amounts: vec![near_sdk::json_types::U128(200)],
        memo: None,
        msg: None,
        storage_deposit: None,
        min_gas: None,
    };

    let mt_withdraw_payload = env.user1.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        defuse2.id(), // Sign for defuse2 since we're simulating on it
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![mt_withdraw_intent.clone().into()],
        },
    );

    // Simulate the intent on defuse2 (which has the tokens)
    let result = defuse2
        .simulate_intents([mt_withdraw_payload.clone()])
        .await
        .unwrap();

    assert_eq!(result.intents_executed.len(), 1);

    // Prepare expected MtWithdraw event
    let expected_log = DefuseEvent::MtWithdraw(Cow::Owned(vec![IntentEvent {
            intent_hash: mt_withdraw_payload.hash(),
            event: AccountEvent {
                account_id: env.user1.id().clone().into(),
                event: Cow::Owned(mt_withdraw_intent),
            },
        }]))
        .as_near_sdk_log();

    result.logs.iter().for_each(|log| println!("{}", log));
    println!("{}", near_sdk::serde_json::to_string_pretty(&result).unwrap());

    assert!(result.logs.iter().any(|log| log == &expected_log));
    result.into_result().unwrap();
}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_storage_deposit_intent(
    #[notrace] mut rng: impl Rng,
) {
    let env = Env::builder()
        .no_registration(true)
        .build()
        .await;

    let wnear_token_id = TokenId::from(Nep141TokenId::new(env.wnear.id().clone()));

    // Step 1: Deposit NEAR to get wNEAR
    let wnear_amount = NearToken::from_millinear(100);
    env.user1
        .near_deposit(env.wnear.id(), wnear_amount)
        .await
        .unwrap();

    // Step 2: Transfer wNEAR to Defuse contract
    env.user1
        .ft_transfer_call(
            env.wnear.id(),
            env.defuse.id(),
            wnear_amount.as_yoctonear(),
            None,
            &env.user1.id().to_string(), // Recipient in Defuse
        )
        .await
        .unwrap();

    // Verify wNEAR balance in Defuse
    assert_eq!(
        env.defuse
            .mt_balance_of(env.user1.id(), &wnear_token_id.to_string())
            .await
            .unwrap(),
        wnear_amount.as_yoctonear()
    );

    let nonce = rng.random();

    // Step 3: Create StorageDeposit intent
    let storage_deposit_amount = NearToken::from_millinear(10);
    let storage_deposit_intent = StorageDeposit {
        contract_id: env.ft1.clone(), // Deposit storage on ft1 contract
        deposit_for_account_id: env.user2.id().clone(), // For user2
        amount: storage_deposit_amount,
    };

    let storage_deposit_payload = env.user1.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![storage_deposit_intent.clone().into()],
        },
    );

    // Step 4: Simulate the intent
    let result = env
        .defuse
        .simulate_intents([storage_deposit_payload.clone()])
        .await
        .unwrap();

    assert_eq!(result.intents_executed.len(), 1);

    // Step 5: Prepare expected StorageDeposit event
    let expected_log = DefuseEvent::StorageDeposit(Cow::Owned(vec![IntentEvent {
            intent_hash: storage_deposit_payload.hash(),
            event: AccountEvent {
                account_id: env.user1.id().clone().into(),
                event: Cow::Owned(storage_deposit_intent),
            },
        }]))
        .as_near_sdk_log();

    result.logs.iter().for_each(|log| println!("{}", log));
    println!("{}", near_sdk::serde_json::to_string_pretty(&result).unwrap());

    assert!(result.logs.iter().any(|log| log == &expected_log));
    result.into_result().unwrap();
}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_token_diff_intent(
    #[notrace] mut rng: impl Rng,
) {
    let env = Env::builder()
        .fee(Pips::ZERO)
        .no_registration(true)
        .build()
        .await;

    let ft1_token_id = TokenId::from(Nep141TokenId::new(env.ft1.clone()));
    let ft2_token_id = TokenId::from(Nep141TokenId::new(env.ft2.clone()));

    // Step 1: Deposit tokens to users
    // user1 has 100 ft1
    env.defuse_ft_deposit_to(&env.ft1, 100, env.user1.id())
        .await
        .unwrap();

    // user2 has 200 ft2
    env.defuse_ft_deposit_to(&env.ft2, 200, env.user2.id())
        .await
        .unwrap();

    // Verify initial balances
    assert_eq!(
        env.defuse
            .mt_balance_of(env.user1.id(), &ft1_token_id.to_string())
            .await
            .unwrap(),
        100
    );
    assert_eq!(
        env.defuse
            .mt_balance_of(env.user2.id(), &ft2_token_id.to_string())
            .await
            .unwrap(),
        200
    );

    let nonce1 = rng.random();
    let nonce2 = rng.random();

    // Step 2: Create TokenDiff intents for P2P swap
    // user1: swap -100 ft1 for +200 ft2
    let user1_token_diff = TokenDiff {
        diff: TokenDeltas::default()
            .with_apply_deltas([
                (ft1_token_id.clone(), -100),
                (ft2_token_id.clone(), 200),
            ])
            .unwrap(),
        memo: None,
        referral: None,
    };

    // user2: swap -200 ft2 for +100 ft1
    let user2_token_diff = TokenDiff {
        diff: TokenDeltas::default()
            .with_apply_deltas([
                (ft1_token_id.clone(), 100),
                (ft2_token_id.clone(), -200),
            ])
            .unwrap(),
        memo: None,
        referral: None,
    };

    // Step 3: Sign both intents
    let user1_payload = env.user1.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        env.defuse.id(),
        nonce1,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![user1_token_diff.clone().into()],
        },
    );

    let user2_payload = env.user2.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        env.defuse.id(),
        nonce2,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![user2_token_diff.clone().into()],
        },
    );

    // Step 4: Simulate both intents
    let result = env
        .defuse
        .simulate_intents([user1_payload.clone(), user2_payload.clone()])
        .await
        .unwrap();

    assert_eq!(result.intents_executed.len(), 2);

    // Step 5: Verify the simulation succeeded (no invariant violation)
    result.logs.iter().for_each(|log| println!("{}", log));
    println!("{}", near_sdk::serde_json::to_string_pretty(&result).unwrap());

    result.into_result().unwrap();
}

// TODO: make sure PublicKeyAdd is recorded in simulation log
#[tokio::test]
#[rstest]
#[trace]
async fn simulate_add_public_key_intent(
    #[notrace] mut rng: impl Rng,
) {
    let env = Env::builder()
        .no_registration(true)
        .build()
        .await;

    let nonce = rng.random();

    // Step 1: Generate a new public key to add
    // We'll use a randomly generated Ed25519 public key
    let mut random_key_bytes = [0u8; 32];
    rng.fill_bytes(&mut random_key_bytes);
    let new_public_key = PublicKey::Ed25519(random_key_bytes);

    // Step 2: Create AddPublicKey intent
    let add_public_key_intent = AddPublicKey {
        public_key: new_public_key,
    };

    let add_public_key_payload = env.user1.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![add_public_key_intent.into()],
        },
    );

    // Step 3: Simulate the intent
    let result = env
        .defuse
        .simulate_intents([add_public_key_payload.clone()])
        .await
        .unwrap();

    assert_eq!(result.intents_executed.len(), 1);

    // Step 4: Verify the simulation succeeded
    // Note: AddPublicKey doesn't emit events through the inspector,
    // so we just verify that the simulation completed successfully
    result.logs.iter().for_each(|log| println!("{}", log));
    println!("{}", near_sdk::serde_json::to_string_pretty(&result).unwrap());

    result.into_result().unwrap();
}

// TODO: make sure PublicKeyEvent is recorded in simulation log
#[tokio::test]
#[rstest]
#[trace]
async fn simulate_remove_public_key_intent(
    #[notrace] mut rng: impl Rng,
) {
    let env = Env::builder()
        .no_registration(true)
        .build()
        .await;

    // Step 1: Generate a new public key to add and then remove
    let mut random_key_bytes = [0u8; 32];
    rng.fill_bytes(&mut random_key_bytes);
    let new_public_key = PublicKey::Ed25519(random_key_bytes);

    // Step 2: First, actually execute AddPublicKey to add the key to the state
    let add_nonce = rng.random();
    let add_public_key_intent = AddPublicKey {
        public_key: new_public_key,
    };

    let add_public_key_payload = env.user1.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        env.defuse.id(),
        add_nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![add_public_key_intent.into()],
        },
    );

    // Execute the add intent (not simulate) to actually add the key
    env.defuse
        .execute_intents([add_public_key_payload])
        .await
        .unwrap();

    // Step 3: Create RemovePublicKey intent to remove the key we just added
    let remove_nonce = rng.random();
    let remove_public_key_intent = RemovePublicKey {
        public_key: new_public_key,
    };

    let remove_public_key_payload = env.user1.sign_defuse_message(
        SigningStandard::arbitrary(&mut Unstructured::new(&rng.random::<[u8; 1]>())).unwrap(),
        env.defuse.id(),
        remove_nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![remove_public_key_intent.into()],
        },
    );

    // Step 4: Simulate the remove intent
    let result = env
        .defuse
        .simulate_intents([remove_public_key_payload.clone()])
        .await
        .unwrap();

    assert_eq!(result.intents_executed.len(), 1);

    // Step 5: Verify the simulation succeeded
    // Note: RemovePublicKey doesn't emit events through the inspector,
    // so we just verify that the simulation completed successfully
    result.logs.iter().for_each(|log| println!("{}", log));
    println!("{}", near_sdk::serde_json::to_string_pretty(&result).unwrap());

    result.into_result().unwrap();
}


