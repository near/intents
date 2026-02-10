use defuse_sandbox::extensions::defuse::{
    account_manager::{AccountManagerExt, AccountViewExt},
    contract::core::{
        accounts::{AccountEvent, NonceEvent, PublicKeyEvent, TransferEvent},
        amounts::Amounts,
        crypto::{Payload, PublicKey},
        events::DefuseEvent,
        fees::{FeesConfig, Pips},
        intents::{
            Intent, IntentEvent,
            account::{AddPublicKey, RemovePublicKey, SetAuthByPredecessorId},
            auth::AuthCall,
            token_diff::{TokenDeltas, TokenDiff, TokenDiffEvent},
            tokens::{
                FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit, Transfer,
                imt::{ImtBurn, ImtMint},
            },
        },
        token_id::{TokenId, nep141::Nep141TokenId, nep171::Nep171TokenId, nep245::Nep245TokenId},
    },
    deployer::DefuseExt,
    intents::{ExecuteIntentsExt, SimulateIntents},
    nonce::ExtractNonceExt,
    signer::DefaultDefuseSignerExt,
};
use defuse::contract::config::{DefuseConfig, RolesConfig};
use defuse_sandbox::extensions::{
    ft::FtExt,
    mt::{MtExt, MtViewExt},
    nft::{NftDeployerExt, NftExt},
    wnear::WNearExt,
};

use crate::{
    env::{DEFUSE_WASM, Env, NON_FUNGIBLE_TOKEN_WASM},
    sandbox::api::types::{json::Base64VecU8, nft::NFTContractMetadata},
    tests::defuse::intents::AccountNonceIntentEvent,
    utils::fixtures::public_key,
};
use near_contract_standards::non_fungible_token::metadata::{NFT_METADATA_SPEC, TokenMetadata};
use near_sdk::{AsNep297Event, NearToken};
use rstest::rstest;
use std::borrow::Cow;

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_transfer_intent() {
    let env = Env::builder().build().await;

    let (user1, user2, ft1) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user1.id(), user2.id()], vec![ft1.id()])
        .await;

    env.defuse_ft_deposit_to(ft1.id(), 1000, user1.id(), None)
        .await
        .unwrap();

    let transfer_intent = Transfer {
        receiver_id: user2.id().clone(),
        tokens: Amounts::new(
            std::iter::once((TokenId::from(Nep141TokenId::new(ft1.id().clone())), 1000)).collect(),
        ),
        memo: None,
        notification: None,
    };

    let transfer_intent_payload = user1
        .sign_defuse_payload_default(&env.defuse, [transfer_intent.clone()])
        .await
        .unwrap();
    let nonce = transfer_intent_payload.extract_nonce().unwrap();

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
                        account_id: user1.id().clone().into(),
                        event: TransferEvent {
                            receiver_id: Cow::Borrowed(&transfer_intent.receiver_id),
                            tokens: transfer_intent.tokens,
                            memo: Cow::Borrowed(&transfer_intent.memo),
                        },
                    },
                }]
                .into()
            )
            .to_nep297_event()
            .to_event_log(),
            AccountNonceIntentEvent::new(&user1.id(), nonce, &transfer_intent_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_ft_withdraw_intent() {
    let env = Env::builder().build().await;

    let (user1, user2, ft1) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user1.id(), user2.id()], vec![ft1.id()])
        .await;

    env.defuse_ft_deposit_to(ft1.id(), 1000, user1.id(), None)
        .await
        .unwrap();

    let ft1_token_id = TokenId::from(Nep141TokenId::new(ft1.id().clone()));

    assert_eq!(
        env.defuse
            .mt_balance_of(user1.id(), &ft1_token_id.to_string())
            .await
            .unwrap(),
        1000
    );

    let ft_withdraw_intent = FtWithdraw {
        token: ft1.id().clone(),
        receiver_id: user2.id().clone(),
        amount: near_sdk::json_types::U128(500),
        memo: None,
        msg: None,
        storage_deposit: None,
        min_gas: None,
    };

    let ft_withdraw_payload = user1
        .sign_defuse_payload_default(&env.defuse, [ft_withdraw_intent.clone()])
        .await
        .unwrap();
    let nonce = ft_withdraw_payload.extract_nonce().unwrap();

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
                    account_id: user1.id().clone().into(),
                    event: Cow::Owned(ft_withdraw_intent),
                },
            }]))
            .to_nep297_event()
            .to_event_log(),
            AccountNonceIntentEvent::new(&user1.id(), nonce, &ft_withdraw_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_native_withdraw_intent() {
    let env = Env::builder().build().await;

    let (user1, user2) = futures::join!(env.create_user(), env.create_user());

    env.initial_ft_storage_deposit(vec![user1.id(), user2.id()], &[])
        .await;

    let wnear_token_id = TokenId::from(Nep141TokenId::new(env.wnear.id().clone()));

    // Deposit wNEAR to user1's Defuse account
    let wnear_amount = NearToken::from_millinear(100);
    user1
        .near_deposit(env.wnear.id(), wnear_amount)
        .await
        .unwrap();

    user1
        .ft_transfer_call(
            env.wnear.id(),
            env.defuse.id(),
            wnear_amount.as_yoctonear(),
            None,
            user1.id(),
        )
        .await
        .unwrap();

    // Verify wNEAR balance in Defuse
    assert_eq!(
        env.defuse
            .mt_balance_of(user1.id(), &wnear_token_id.to_string())
            .await
            .unwrap(),
        wnear_amount.as_yoctonear()
    );

    let withdraw_amount = NearToken::from_millinear(50);
    let native_withdraw_intent = NativeWithdraw {
        receiver_id: user2.id().clone(),
        amount: withdraw_amount,
    };

    let native_withdraw_payload = user1
        .sign_defuse_payload_default(&env.defuse, [native_withdraw_intent.clone()])
        .await
        .unwrap();
    let nonce = native_withdraw_payload.extract_nonce().unwrap();

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
                    account_id: user1.id().clone().into(),
                    event: Cow::Owned(native_withdraw_intent),
                },
            }]))
            .to_nep297_event()
            .to_event_log(),
            AccountNonceIntentEvent::new(&user1.id(), nonce, &native_withdraw_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}

pub const DUMMY_NFT_URL: &str = "http://example.com/nft/";
pub const DUMMY_NFT_REFERENCE_HASH: [u8; 32] = [13; 32];
pub const DUMMY_NFT_ID: &str = "thisisdummynftid";

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_nft_withdraw_intent() {
    let env = Env::builder().build().await;

    let (user1, user2) =
        futures::join!(env.create_named_user("nft_issuer_admin"), env.create_user());

    env.tx(user1.id())
        .transfer(NearToken::from_near(100))
        .await
        .unwrap();

    let nft_contract = user1
        .deploy_vanilla_nft_issuer(
            "nft1",
            user1.id(),
            NFTContractMetadata {
                reference: Some(DUMMY_NFT_URL.to_string()),
                reference_hash: Some(Base64VecU8(DUMMY_NFT_REFERENCE_HASH.to_vec())),
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Test NFT".to_string(),
                symbol: "TNFT".to_string(),
                icon: None,
                base_uri: None,
            },
            NON_FUNGIBLE_TOKEN_WASM.clone(),
        )
        .await
        .unwrap();

    let _nft = user1
        .nft_mint(
            nft_contract.id(),
            &DUMMY_NFT_ID.to_string(),
            user1.id(),
            &TokenMetadata::default(),
        )
        .await
        .unwrap();

    let nft_token_id: TokenId =
        Nep171TokenId::new(nft_contract.id().to_owned(), DUMMY_NFT_ID.to_string()).into();

    user1
        .nft_transfer_call(
            nft_contract.id(),
            env.defuse.id(),
            DUMMY_NFT_ID.to_string(),
            None,
            user1.id().to_string(),
        )
        .await
        .unwrap();

    assert_eq!(
        env.defuse
            .mt_balance_of(user1.id(), &nft_token_id.to_string())
            .await
            .unwrap(),
        1
    );

    let nft_withdraw_intent = NftWithdraw {
        token: nft_contract.id().clone(),
        receiver_id: user2.id().clone(),
        token_id: DUMMY_NFT_ID.to_string(),
        memo: None,
        msg: None,
        storage_deposit: None,
        min_gas: None,
    };

    let nft_withdraw_payload = user1
        .sign_defuse_payload_default(&env.defuse, [nft_withdraw_intent.clone()])
        .await
        .unwrap();
    let nonce = nft_withdraw_payload.extract_nonce().unwrap();

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
                    account_id: user1.id().clone().into(),
                    event: Cow::Owned(nft_withdraw_intent),
                },
            }]))
            .to_nep297_event()
            .to_event_log(),
            AccountNonceIntentEvent::new(&user1.id(), nonce, &nft_withdraw_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_mt_withdraw_intent() {
    let env = Env::builder().build().await;

    let (user1, user2, ft1) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

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
            DEFUSE_WASM.clone(),
        )
        .await
        .unwrap();

    env.initial_ft_storage_deposit(vec![user1.id(), user2.id()], vec![ft1.id()])
        .await;

    // Register user1's public key on defuse2
    user1
        .add_public_key(
            defuse2.id(),
            &user1.signer().get_public_key().await.unwrap().into(),
        )
        .await
        .unwrap();

    let ft1_id = TokenId::from(Nep141TokenId::new(ft1.id().clone()));

    // Step 1: Deposit FT to user1 in the first Defuse contract (stored as MT internally)
    env.defuse_ft_deposit_to(ft1.id(), 1000, user1.id(), None)
        .await
        .unwrap();

    // Verify balance in first Defuse contract
    assert_eq!(
        env.defuse
            .mt_balance_of(user1.id(), &ft1_id.to_string())
            .await
            .unwrap(),
        1000
    );

    // Step 2: Transfer tokens from defuse1 to defuse2 using mt_transfer_call
    // This creates MT tokens in defuse2 that can be withdrawn
    user1
        .mt_transfer_call(
            env.defuse.id(),
            defuse2.id(),
            ft1_id.to_string(),
            500,
            None,
            user1.id().to_string(), // user1 will own these tokens in defuse2
        )
        .await
        .unwrap();

    // Verify tokens are now in defuse2 as NEP-245 tokens
    let nep245_token_id: TokenId =
        Nep245TokenId::new(env.defuse.id().to_owned(), ft1_id.to_string()).into();

    assert_eq!(
        defuse2
            .mt_balance_of(user1.id(), &nep245_token_id.to_string())
            .await
            .unwrap(),
        500
    );

    // Step 3: Create MtWithdraw intent to withdraw MT tokens from defuse2 back to defuse1
    // Now we're simulating on defuse2, withdrawing to defuse1
    let mt_withdraw_intent = MtWithdraw {
        token: env.defuse.id().clone(),  // External NEP-245 contract (defuse1)
        receiver_id: user2.id().clone(), // Withdraw to user2's account in defuse1
        token_ids: vec![ft1_id.to_string()], // The FT token ID within defuse1
        amounts: vec![near_sdk::json_types::U128(200)],
        memo: None,
        msg: None,
        storage_deposit: None,
        min_gas: None,
    };

    let mt_withdraw_payload = user1
        .sign_defuse_payload_default(&defuse2, [mt_withdraw_intent.clone()])
        .await
        .unwrap();
    let nonce = mt_withdraw_payload.extract_nonce().unwrap();

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
                    account_id: user1.id().clone().into(),
                    event: Cow::Owned(mt_withdraw_intent),
                },
            }]))
            .to_nep297_event()
            .to_event_log(),
            AccountNonceIntentEvent::new(&user1.id(), nonce, &mt_withdraw_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_storage_deposit_intent() {
    let env = Env::builder().build().await;

    let (user1, user2, ft1) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user1.id()], vec![ft1.id()])
        .await;

    let wnear_token_id = TokenId::from(Nep141TokenId::new(env.wnear.id().clone()));

    let wnear_amount = NearToken::from_millinear(100);
    user1
        .near_deposit(env.wnear.id(), wnear_amount)
        .await
        .unwrap();

    user1
        .ft_transfer_call(
            env.wnear.id(),
            env.defuse.id(),
            wnear_amount.as_yoctonear(),
            None,
            user1.id(), // Recipient in Defuse
        )
        .await
        .unwrap();

    // Verify wNEAR balance in Defuse
    assert_eq!(
        env.defuse
            .mt_balance_of(user1.id(), &wnear_token_id.to_string())
            .await
            .unwrap(),
        wnear_amount.as_yoctonear()
    );

    let storage_deposit_amount = NearToken::from_millinear(10);
    let storage_deposit_intent = StorageDeposit {
        contract_id: ft1.id().clone(), // Deposit storage on ft1 contract
        deposit_for_account_id: user2.id().clone(), // For user2
        amount: storage_deposit_amount,
    };

    let storage_deposit_payload = user1
        .sign_defuse_payload_default(&env.defuse, [storage_deposit_intent.clone()])
        .await
        .unwrap();
    let nonce = storage_deposit_payload.extract_nonce().unwrap();

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
                    account_id: user1.id().clone().into(),
                    event: Cow::Owned(storage_deposit_intent),
                },
            }]))
            .to_nep297_event()
            .to_event_log(),
            AccountNonceIntentEvent::new(&user1.id(), nonce, &storage_deposit_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_token_diff_intent() {
    let env = Env::builder().fee(Pips::ZERO).build().await;

    let (user1, user2, ft1, ft2) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token()
    );

    env.initial_ft_storage_deposit(vec![user1.id(), user2.id()], vec![ft1.id(), ft2.id()])
        .await;

    let ft1_token_id = TokenId::from(Nep141TokenId::new(ft1.id().clone()));
    let ft2_token_id = TokenId::from(Nep141TokenId::new(ft2.id().clone()));

    // user1 has 100 ft1
    env.defuse_ft_deposit_to(ft1.id(), 100, user1.id(), None)
        .await
        .unwrap();

    // user2 has 200 ft2
    env.defuse_ft_deposit_to(ft2.id(), 200, user2.id(), None)
        .await
        .unwrap();

    // Verify initial balances
    assert_eq!(
        env.defuse
            .mt_balance_of(user1.id(), &ft1_token_id.to_string())
            .await
            .unwrap(),
        100
    );
    assert_eq!(
        env.defuse
            .mt_balance_of(user2.id(), &ft2_token_id.to_string())
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

    let user1_payload = user1
        .sign_defuse_payload_default(&env.defuse, [user1_token_diff.clone()])
        .await
        .unwrap();
    let nonce1 = user1_payload.extract_nonce().unwrap();

    let user2_payload = user2
        .sign_defuse_payload_default(&env.defuse, [user2_token_diff.clone()])
        .await
        .unwrap();
    let nonce2 = user2_payload.extract_nonce().unwrap();

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
                    account_id: user1.id().clone().into(),
                    event: TokenDiffEvent {
                        diff: Cow::Owned(user1_token_diff),
                        fees_collected: Amounts::default(),
                    },
                },
            }]))
            .to_nep297_event()
            .to_event_log(),
            DefuseEvent::TokenDiff(Cow::Owned(vec![IntentEvent {
                intent_hash: user2_payload.hash(),
                event: AccountEvent {
                    account_id: user2.id().clone().into(),
                    event: TokenDiffEvent {
                        diff: Cow::Owned(user2_token_diff),
                        fees_collected: Amounts::default(),
                    },
                },
            }]))
            .to_nep297_event()
            .to_event_log(),
            DefuseEvent::IntentsExecuted(
                vec![
                    IntentEvent::new(
                        AccountEvent::new(user1.id(), NonceEvent::new(nonce1)),
                        user1_payload.hash()
                    ),
                    IntentEvent::new(
                        AccountEvent::new(user2.id(), NonceEvent::new(nonce2)),
                        user2_payload.hash()
                    ),
                ]
                .into()
            )
            .to_nep297_event()
            .to_event_log(),
        ]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_add_public_key_intent(public_key: PublicKey) {
    let env = Env::builder().build().await;

    let user1 = env.create_user().await;

    let new_public_key = public_key;

    let add_public_key_intent = AddPublicKey {
        public_key: new_public_key,
    };

    let add_public_key_payload = user1
        .sign_defuse_payload_default(&env.defuse, [add_public_key_intent])
        .await
        .unwrap();
    let nonce = add_public_key_payload.extract_nonce().unwrap();

    let result = env
        .defuse
        .simulate_intents([add_public_key_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::PublicKeyAdded(AccountEvent::new(
                user1.id(),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&new_public_key)
                },
            ))
            .to_nep297_event()
            .to_event_log(),
            AccountNonceIntentEvent::new(&user1.id(), nonce, &add_public_key_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_remove_public_key_intent(public_key: PublicKey) {
    let env = Env::builder().build().await;

    let user1 = env.create_user().await;

    let new_public_key = public_key;
    let add_public_key_intent = AddPublicKey {
        public_key: new_public_key,
    };

    let add_public_key_payload = user1
        .sign_defuse_payload_default(&env.defuse, [add_public_key_intent])
        .await
        .unwrap();

    // Execute the add intent (not simulate) to actually add the key
    env.execute_intents(env.defuse.id(), [add_public_key_payload])
        .await
        .unwrap();

    let remove_public_key_intent = RemovePublicKey {
        public_key: new_public_key,
    };

    let remove_public_key_payload = user1
        .sign_defuse_payload_default(&env.defuse, [remove_public_key_intent])
        .await
        .unwrap();
    let remove_nonce = remove_public_key_payload.extract_nonce().unwrap();

    let result = env
        .defuse
        .simulate_intents([remove_public_key_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::PublicKeyRemoved(AccountEvent::new(
                user1.id(),
                PublicKeyEvent {
                    public_key: Cow::Borrowed(&new_public_key)
                },
            ))
            .to_nep297_event()
            .to_event_log(),
            AccountNonceIntentEvent::new(&user1.id(), remove_nonce, &remove_public_key_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_set_auth_by_predecessor_id_intent() {
    let env = Env::builder().build().await;

    let user1 = env.create_user().await;

    let set_auth_intent = SetAuthByPredecessorId { enabled: true };

    let set_auth_payload = user1
        .sign_defuse_payload_default(&env.defuse, [set_auth_intent.clone()])
        .await
        .unwrap();
    let nonce = set_auth_payload.extract_nonce().unwrap();

    let result = env
        .defuse
        .simulate_intents([set_auth_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::SetAuthByPredecessorId(AccountEvent::new(user1.id(), set_auth_intent,))
                .to_nep297_event()
                .to_event_log(),
            AccountNonceIntentEvent::new(&user1.id(), nonce, &set_auth_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_auth_call_intent() {
    let env = Env::builder().build().await;

    let (user1, ft1) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user1.id()], vec![ft1.id()])
        .await;

    let wnear_token_id = TokenId::from(Nep141TokenId::new(env.wnear.id().clone()));

    let wnear_amount = NearToken::from_millinear(100);

    user1
        .near_deposit(env.wnear.id(), wnear_amount)
        .await
        .unwrap();
    user1
        .ft_transfer_call(
            env.wnear.id(),
            env.defuse.id(),
            wnear_amount.as_yoctonear(),
            None,
            user1.id(),
        )
        .await
        .unwrap();

    // Verify wNEAR balance
    assert_eq!(
        env.defuse
            .mt_balance_of(user1.id(), &wnear_token_id.to_string())
            .await
            .unwrap(),
        wnear_amount.as_yoctonear()
    );

    let auth_call_intent = AuthCall {
        contract_id: ft1.id().clone(), // Call to ft1 contract
        state_init: None,
        msg: "test_message".to_string(),
        attached_deposit: NearToken::from_millinear(10),
        min_gas: None,
    };

    let auth_call_payload = user1
        .sign_defuse_payload_default(&env.defuse, [auth_call_intent])
        .await
        .unwrap();

    let nonce = auth_call_payload.extract_nonce().unwrap();

    let result = env
        .defuse
        .simulate_intents([auth_call_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            AccountNonceIntentEvent::new(&user1.id(), nonce, &auth_call_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_mint_intent() {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let token_id = "sometoken.near".to_string();
    let memo = "Some memo";
    let amount = 1000;

    let mint_intent = ImtMint {
        tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
        memo: Some(memo.to_string()),
        receiver_id: user.id().clone(),
        notification: None,
    };

    let mint_payload = user
        .sign_defuse_payload_default(&env.defuse, [mint_intent.clone()])
        .await
        .unwrap();

    let nonce = mint_payload.extract_nonce().unwrap();

    let result = env
        .defuse
        .simulate_intents([mint_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::ImtMint(Cow::Owned(vec![IntentEvent {
                intent_hash: mint_payload.hash(),
                event: AccountEvent {
                    account_id: user.id().clone().into(),
                    event: Cow::Owned(mint_intent)
                },
            }]))
            .to_nep297_event()
            .to_event_log(),
            AccountNonceIntentEvent::new(&user.id(), nonce, &mint_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_burn_intent() {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let token_id = "sometoken.near".to_string();
    let memo = "Some memo";
    let amount = 1000;

    let mint_payload = user
        .sign_defuse_payload_default(
            &env.defuse,
            [ImtMint {
                tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
                memo: Some(memo.to_string()),
                receiver_id: user.id().clone(),
                notification: None,
            }],
        )
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [mint_payload])
        .await
        .unwrap();

    let burn_intent = ImtBurn {
        minter_id: user.id().clone(),
        tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
        memo: Some(memo.to_string()),
    };

    let burn_payload = user
        .sign_defuse_payload_default(&env.defuse, [burn_intent.clone()])
        .await
        .unwrap();
    let nonce = burn_payload.extract_nonce().unwrap();

    let result = env
        .defuse
        .simulate_intents([burn_payload.clone()])
        .await
        .unwrap();

    assert_eq!(
        result.report.logs,
        vec![
            DefuseEvent::ImtBurn(Cow::Owned(vec![IntentEvent {
                intent_hash: burn_payload.hash(),
                event: AccountEvent {
                    account_id: user.id().clone().into(),
                    event: Cow::Owned(burn_intent)
                },
            }]))
            .to_nep297_event()
            .to_event_log(),
            AccountNonceIntentEvent::new(&user.id(), nonce, &burn_payload)
                .into_event()
                .to_nep297_event()
                .to_event_log(),
        ]
    );
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulation_fails_on_used_nonce() {
    let env = Env::builder().build().await;

    let user = env.create_user().await;

    let payload = user
        .sign_defuse_payload_default(&env.defuse, Vec::<Intent>::new())
        .await
        .unwrap();

    env.execute_intents(env.defuse.id(), [payload.clone()])
        .await
        .unwrap();

    assert!(
        env.defuse
            .is_nonce_used(user.id(), &payload.extract_nonce().unwrap())
            .await
            .unwrap(),
    );

    let result = env.defuse.simulate_intents([payload]).await;

    assert!(result.is_err());
}
