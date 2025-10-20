use crate::tests::defuse::SigningStandard;
use crate::tests::defuse::intents::{AccountNonceIntentEvent, ExecuteIntentsExt, NonceEvent};
use crate::utils::fixtures::{nonce, public_key, signing_standard};
use crate::utils::{ft::FtExt, mt::MtExt, nft::NftExt, wnear::WNearExt};
use crate::{
    tests::defuse::accounts::AccountManagerExt,
    tests::defuse::env::Env,
    tests::defuse::{DefuseExt, DefuseSigner},
};
use defuse::contract::config::{DefuseConfig, RolesConfig};
use defuse::core::crypto::{Payload, PublicKey};
use defuse::core::fees::{FeesConfig, Pips};
use defuse::core::token_id::TokenId;
use defuse::core::token_id::nep141::Nep141TokenId;
use defuse::core::token_id::nep171::Nep171TokenId;
use defuse::core::token_id::nep245::Nep245TokenId;
use defuse::core::{
    Deadline, Nonce,
    accounts::{AccountEvent, PublicKeyEvent},
    amounts::Amounts,
    events::DefuseEvent,
    intents::{
        DefuseIntents, IntentEvent,
        account::{AddPublicKey, RemovePublicKey, SetAuthByPredecessorId},
        auth::AuthCall,
        token_diff::{TokenDeltas, TokenDiff, TokenDiffEvent},
        tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit, Transfer},
    },
};
use defuse_near_utils::NearSdkLog;
use defuse_randomness::Rng;
use defuse_test_utils::random::{gen_random_string, random_bytes, rng};
use near_contract_standards::non_fungible_token::metadata::{
    NFT_METADATA_SPEC, NFTContractMetadata, TokenMetadata,
};
use near_crypto::SecretKey;
use near_sdk::{NearToken, json_types::Base64VecU8};
use rstest::rstest;
use std::borrow::Cow;

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_transfer_intent(nonce: Nonce, signing_standard: SigningStandard) {
    let env = Env::builder().no_registration(true).build().await;

    env.defuse_ft_deposit_to(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();
    let transfer_intent = Transfer {
        receiver_id: env.user2.id().clone(),
        tokens: Amounts::new(
            std::iter::once((TokenId::from(Nep141TokenId::new(env.ft1.clone())), 1000)).collect(),
        ),
        memo: None,
    };

    let transfer_intent_payload = env.user1.sign_defuse_message(
        signing_standard,
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

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::Transfer(
                vec![IntentEvent {
                    intent_hash: transfer_intent_payload.hash(),
                    event: AccountEvent {
                        account_id: env.user1.id().clone().into(),
                        event: Cow::Owned(transfer_intent),
                    },
                }]
                .into()
            )
            .to_near_sdk_log(),
            AccountNonceIntentEvent::new(&env.user1.id(), nonce, &transfer_intent_payload)
                .into_event_log(),
        ]
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_ft_withdraw_intent(nonce: Nonce, signing_standard: SigningStandard) {
    let env = Env::builder().no_registration(true).build().await;

    env.defuse_ft_deposit_to(&env.ft1, 1000, env.user1.id())
        .await
        .unwrap();

    let ft1_token_id = TokenId::from(Nep141TokenId::new(env.ft1.clone()));

    assert_eq!(
        env.defuse
            .mt_balance_of(env.user1.id(), &ft1_token_id.to_string())
            .await
            .unwrap(),
        1000
    );

    let ft_withdraw_intent = FtWithdraw {
        token: env.ft1.clone(),
        receiver_id: env.user2.id().clone(),
        amount: near_sdk::json_types::U128(500),
        memo: None,
        msg: None,
        storage_deposit: None,
        min_gas: None,
    };

    let ft_withdraw_payload = env.user1.sign_defuse_message(
        signing_standard,
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![ft_withdraw_intent.clone().into()],
        },
    );

    let result = env
        .defuse
        .simulate_intents([ft_withdraw_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::FtWithdraw(Cow::Owned(vec![IntentEvent {
                intent_hash: ft_withdraw_payload.hash(),
                event: AccountEvent {
                    account_id: env.user1.id().clone().into(),
                    event: Cow::Owned(ft_withdraw_intent),
                },
            }]))
            .to_near_sdk_log(),
            AccountNonceIntentEvent::new(&env.user1.id(), nonce, &ft_withdraw_payload)
                .into_event_log(),
        ]
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_native_withdraw_intent(nonce: Nonce, signing_standard: SigningStandard) {
    let env = Env::builder().no_registration(true).build().await;

    let wnear_token_id = TokenId::from(Nep141TokenId::new(env.wnear.id().clone()));

    // Deposit wNEAR to user1's Defuse account
    let wnear_amount = NearToken::from_millinear(100);
    env.user1
        .near_deposit(env.wnear.id(), wnear_amount)
        .await
        .unwrap();

    env.user1
        .ft_transfer_call(
            env.wnear.id(),
            env.defuse.id(),
            wnear_amount.as_yoctonear(),
            None,
            env.user1.id().as_ref(),
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

    let withdraw_amount = NearToken::from_millinear(50);
    let native_withdraw_intent = NativeWithdraw {
        receiver_id: env.user2.id().clone(),
        amount: withdraw_amount,
    };

    let native_withdraw_payload = env.user1.sign_defuse_message(
        signing_standard,
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![native_withdraw_intent.clone().into()],
        },
    );

    let result = env
        .defuse
        .simulate_intents([native_withdraw_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::NativeWithdraw(Cow::Owned(vec![IntentEvent {
                intent_hash: native_withdraw_payload.hash(),
                event: AccountEvent {
                    account_id: env.user1.id().clone().into(),
                    event: Cow::Owned(native_withdraw_intent),
                },
            }]))
            .to_near_sdk_log(),
            AccountNonceIntentEvent::new(&env.user1.id(), nonce, &native_withdraw_payload)
                .into_event_log(),
        ]
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_nft_withdraw_intent(
    #[notrace] mut rng: impl Rng,
    nonce: Nonce,
    signing_standard: SigningStandard,
) {
    let env = Env::builder().no_registration(true).build().await;

    env.transfer_near(env.user1.id(), NearToken::from_near(100))
        .await
        .unwrap()
        .unwrap();

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

    let nft_id = gen_random_string(&mut rng, 32..=32);
    let _nft = env
        .user1
        .nft_mint(
            nft_contract.id(),
            &nft_id,
            env.user1.id(),
            &TokenMetadata::default(),
        )
        .await
        .unwrap();

    let nft_token_id =
        TokenId::from(Nep171TokenId::new(nft_contract.id().to_owned(), nft_id.clone()).unwrap());

    env.user1
        .nft_transfer_call(
            nft_contract.id(),
            env.defuse.id(),
            nft_id.clone(),
            None,
            env.user1.id().to_string(),
        )
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(env.user1.id(), &nft_token_id.to_string())
            .await
            .unwrap(),
        1
    );

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
        signing_standard,
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![nft_withdraw_intent.clone().into()],
        },
    );

    let result = env
        .defuse
        .simulate_intents([nft_withdraw_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::NftWithdraw(Cow::Owned(vec![IntentEvent {
                intent_hash: nft_withdraw_payload.hash(),
                event: AccountEvent {
                    account_id: env.user1.id().clone().into(),
                    event: Cow::Owned(nft_withdraw_intent),
                },
            }]))
            .to_near_sdk_log(),
            AccountNonceIntentEvent::new(&env.user1.id(), nonce, &nft_withdraw_payload)
                .into_event_log(),
        ]
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_mt_withdraw_intent(nonce: Nonce, signing_standard: SigningStandard) {
    let env = Env::builder().build().await;

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
    let user1_secret_key: SecretKey = env.user1.secret_key().to_string().parse().unwrap();
    if let near_crypto::PublicKey::ED25519(pk) = user1_secret_key.public_key() {
        env.user1
            .add_public_key(defuse2.id(), PublicKey::Ed25519(pk.0))
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
    let nep245_token_id =
        TokenId::from(Nep245TokenId::new(env.defuse.id().to_owned(), ft1.to_string()).unwrap());

    assert_eq!(
        defuse2
            .mt_balance_of(env.user1.id(), &nep245_token_id.to_string())
            .await
            .unwrap(),
        500
    );

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
        signing_standard,
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

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::MtWithdraw(Cow::Owned(vec![IntentEvent {
                intent_hash: mt_withdraw_payload.hash(),
                event: AccountEvent {
                    account_id: env.user1.id().clone().into(),
                    event: Cow::Owned(mt_withdraw_intent),
                },
            }]))
            .to_near_sdk_log(),
            AccountNonceIntentEvent::new(&env.user1.id(), nonce, &mt_withdraw_payload)
                .into_event_log(),
        ]
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_storage_deposit_intent(nonce: Nonce, signing_standard: SigningStandard) {
    let env = Env::builder().no_registration(true).build().await;

    let wnear_token_id = TokenId::from(Nep141TokenId::new(env.wnear.id().clone()));

    let wnear_amount = NearToken::from_millinear(100);
    env.user1
        .near_deposit(env.wnear.id(), wnear_amount)
        .await
        .unwrap();

    env.user1
        .ft_transfer_call(
            env.wnear.id(),
            env.defuse.id(),
            wnear_amount.as_yoctonear(),
            None,
            env.user1.id().as_ref(), // Recipient in Defuse
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

    let storage_deposit_amount = NearToken::from_millinear(10);
    let storage_deposit_intent = StorageDeposit {
        contract_id: env.ft1.clone(), // Deposit storage on ft1 contract
        deposit_for_account_id: env.user2.id().clone(), // For user2
        amount: storage_deposit_amount,
    };

    let storage_deposit_payload = env.user1.sign_defuse_message(
        signing_standard,
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![storage_deposit_intent.clone().into()],
        },
    );

    let result = env
        .defuse
        .simulate_intents([storage_deposit_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::StorageDeposit(Cow::Owned(vec![IntentEvent {
                intent_hash: storage_deposit_payload.hash(),
                event: AccountEvent {
                    account_id: env.user1.id().clone().into(),
                    event: Cow::Owned(storage_deposit_intent),
                },
            }]))
            .to_near_sdk_log(),
            AccountNonceIntentEvent::new(&env.user1.id(), nonce, &storage_deposit_payload)
                .into_event_log(),
        ]
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_token_diff_intent(
    #[from(nonce)] nonce1: Nonce,
    #[from(nonce)] nonce2: Nonce,
    #[from(signing_standard)] signing_standard1: SigningStandard,
    #[from(signing_standard)] signing_standard2: SigningStandard,
) {
    let env = Env::builder()
        .fee(Pips::ZERO)
        .no_registration(true)
        .build()
        .await;

    let ft1_token_id = TokenId::from(Nep141TokenId::new(env.ft1.clone()));
    let ft2_token_id = TokenId::from(Nep141TokenId::new(env.ft2.clone()));

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

    // user1: swap -100 ft1 for +200 ft2
    let user1_token_diff = TokenDiff {
        diff: TokenDeltas::default()
            .with_apply_deltas([(ft1_token_id.clone(), -100), (ft2_token_id.clone(), 200)])
            .unwrap(),
        memo: None,
        referral: None,
    };

    // user2: swap -200 ft2 for +100 ft1
    let user2_token_diff = TokenDiff {
        diff: TokenDeltas::default()
            .with_apply_deltas([(ft1_token_id.clone(), 100), (ft2_token_id.clone(), -200)])
            .unwrap(),
        memo: None,
        referral: None,
    };

    let user1_payload = env.user1.sign_defuse_message(
        signing_standard1,
        env.defuse.id(),
        nonce1,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![user1_token_diff.clone().into()],
        },
    );

    let user2_payload = env.user2.sign_defuse_message(
        signing_standard2,
        env.defuse.id(),
        nonce2,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![user2_token_diff.clone().into()],
        },
    );

    let result = env
        .defuse
        .simulate_intents([user1_payload.clone(), user2_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::TokenDiff(Cow::Owned(vec![IntentEvent {
                intent_hash: user1_payload.hash(),
                event: AccountEvent {
                    account_id: env.user1.id().clone().into(),
                    event: TokenDiffEvent {
                        diff: Cow::Owned(user1_token_diff),
                        fees_collected: Amounts::default(),
                    },
                },
            }]))
            .to_near_sdk_log(),
            DefuseEvent::TokenDiff(Cow::Owned(vec![IntentEvent {
                intent_hash: user2_payload.hash(),
                event: AccountEvent {
                    account_id: env.user2.id().clone().into(),
                    event: TokenDiffEvent {
                        diff: Cow::Owned(user2_token_diff),
                        fees_collected: Amounts::default(),
                    },
                },
            }]))
            .to_near_sdk_log(),
            DefuseEvent::IntentsExecuted(
                vec![
                    IntentEvent::new(
                        AccountEvent::new(env.user1.id(), NonceEvent::new(nonce1)),
                        user1_payload.hash()
                    ),
                    IntentEvent::new(
                        AccountEvent::new(env.user2.id(), NonceEvent::new(nonce2)),
                        user2_payload.hash()
                    ),
                ]
                .into()
            )
            .to_near_sdk_log(),
        ]
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_add_public_key_intent(
    nonce: Nonce,
    public_key: PublicKey,
    signing_standard: SigningStandard,
) {
    let env = Env::builder().build().await;

    let new_public_key = public_key;

    let add_public_key_intent = AddPublicKey {
        public_key: new_public_key,
    };

    let add_public_key_payload = env.user1.sign_defuse_message(
        signing_standard,
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![add_public_key_intent.into()],
        },
    );

    let result = env
        .defuse
        .simulate_intents([add_public_key_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::PublicKeyAdded(AccountEvent::new(
                env.user1.id(),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&new_public_key)
                },
            ))
            .to_near_sdk_log(),
            AccountNonceIntentEvent::new(&env.user1.id(), nonce, &add_public_key_payload)
                .into_event_log(),
        ]
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_remove_public_key_intent(
    #[from(nonce)] add_nonce: Nonce,
    #[from(nonce)] remove_nonce: Nonce,
    public_key: PublicKey,
    #[from(signing_standard)] add_signing_standard: SigningStandard,
    #[from(signing_standard)] remove_signing_standard: SigningStandard,
) {
    let env = Env::builder().build().await;

    let new_public_key = public_key;
    let add_public_key_intent = AddPublicKey {
        public_key: new_public_key,
    };

    let add_public_key_payload = env.user1.sign_defuse_message(
        add_signing_standard,
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

    let remove_public_key_intent = RemovePublicKey {
        public_key: new_public_key,
    };

    let remove_public_key_payload = env.user1.sign_defuse_message(
        remove_signing_standard,
        env.defuse.id(),
        remove_nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![remove_public_key_intent.into()],
        },
    );

    let result = env
        .defuse
        .simulate_intents([remove_public_key_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::PublicKeyRemoved(AccountEvent::new(
                env.user1.id(),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&new_public_key)
                },
            ))
            .to_near_sdk_log(),
            AccountNonceIntentEvent::new(&env.user1.id(), remove_nonce, &remove_public_key_payload)
                .into_event_log(),
        ]
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_set_auth_by_predecessor_id_intent(
    nonce: Nonce,
    signing_standard: SigningStandard,
) {
    let env = Env::builder().build().await;

    let set_auth_intent = SetAuthByPredecessorId { enabled: true };

    let set_auth_payload = env.user1.sign_defuse_message(
        signing_standard,
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![set_auth_intent.clone().into()],
        },
    );

    let result = env
        .defuse
        .simulate_intents([set_auth_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::SetAuthByPredecessorId(
                AccountEvent::new(env.user1.id(), set_auth_intent,)
            )
            .to_near_sdk_log(),
            AccountNonceIntentEvent::new(&env.user1.id(), nonce, &set_auth_payload)
                .into_event_log(),
        ]
    );
}

#[tokio::test]
#[rstest]
#[trace]
async fn simulate_auth_call_intent(nonce: Nonce, signing_standard: SigningStandard) {
    let env = Env::builder().build().await;

    let wnear_token_id = TokenId::from(Nep141TokenId::new(env.wnear.id().clone()));

    let wnear_amount = NearToken::from_millinear(100);
    env.user1
        .near_deposit(env.wnear.id(), wnear_amount)
        .await
        .unwrap();

    env.user1
        .ft_transfer_call(
            env.wnear.id(),
            env.defuse.id(),
            wnear_amount.as_yoctonear(),
            None,
            env.user1.id().as_ref(),
        )
        .await
        .unwrap();

    // Verify wNEAR balance
    assert_eq!(
        env.defuse
            .mt_balance_of(env.user1.id(), &wnear_token_id.to_string())
            .await
            .unwrap(),
        wnear_amount.as_yoctonear()
    );

    let auth_call_intent = AuthCall {
        contract_id: env.ft1.clone(), // Call to ft1 contract
        msg: "test_message".to_string(),
        attached_deposit: NearToken::from_millinear(10),
        min_gas: None,
    };

    let auth_call_payload = env.user1.sign_defuse_message(
        signing_standard,
        env.defuse.id(),
        nonce,
        Deadline::MAX,
        DefuseIntents {
            intents: vec![auth_call_intent.into()],
        },
    );

    let result = env
        .defuse
        .simulate_intents([auth_call_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            AccountNonceIntentEvent::new(&env.user1.id(), nonce, &auth_call_payload)
                .into_event_log(),
        ]
    );
}
