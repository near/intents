use defuse_sandbox::{
    assert_eq_defuse_event_logs,
    extensions::{
        DEFAULT_GAS,
        defuse::{
            Defuse, DefuseDeployerExt, DefuseExt, DefuseSignerExt, ExtractNonceExt,
            IsNonceUsedArgs, MultiPayloadArgs, ToEventLog,
            contract::config::{DefuseConfig, RolesConfig},
            core::{
                PublicKey,
                amounts::Amounts,
                fees::{FeesConfig, Pips},
                intents::{
                    Intent,
                    account::{AddPublicKey, RemovePublicKey, SetAuthByPredecessorId},
                    auth::AuthCall,
                    imt::{ImtBurn, ImtMint},
                    token_diff::{TokenDeltas, TokenDiff},
                    tokens::{
                        FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit,
                        Transfer,
                    },
                },
                token_id::{
                    TokenId, nep141::Nep141TokenId, nep171::Nep171TokenId, nep245::Nep245TokenId,
                },
            },
            tokens::DepositMessage,
        },
        mt::{Mt, MtBalanceOfArgs, MtExt},
        nft::NftAdminExt,
        wnear::WNearExt,
    },
    kit::{Final, NearToken},
};

use crate::{
    tests::defuse::env::{Env, env},
    utils::fixtures::public_key,
};
use defuse_test_utils::wasms::{DEFUSE_WASM, NON_FUNGIBLE_TOKEN_WASM};
use near_contract_standards::non_fungible_token::metadata::{
    NFT_METADATA_SPEC, NFTContractMetadata,
};
use near_sdk::json_types::Base64VecU8;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn simulate_transfer_intent(#[future(awt)] env: Env) {
    let (user1, user2, ft1) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(
        vec![user1.account_id(), user2.account_id()],
        vec![ft1.contract_id()],
    )
    .await;

    env.defuse_ft_deposit_to(ft1.contract_id(), 1000, user1.account_id(), None)
        .await
        .unwrap();

    let transfer_intent = Transfer {
        receiver_id: user2.account_id().clone(),
        tokens: Amounts::new(
            std::iter::once((
                TokenId::from(Nep141TokenId::new(ft1.contract_id().clone())),
                1000,
            ))
            .collect(),
        ),
        memo: None,
        notification: None,
    };

    let transfer_intent_payload = user1
        .sign_defuse_payload_default(&env.defuse, [transfer_intent.clone()])
        .await
        .unwrap();

    let result = env
        .defuse
        .simulate_intents(MultiPayloadArgs {
            signed: vec![transfer_intent_payload.clone()],
        })
        .await
        .unwrap();

    assert_eq_defuse_event_logs(transfer_intent_payload.to_event_log(), result.report.logs);
}

#[rstest]
#[tokio::test]
async fn simulate_ft_withdraw_intent(#[future(awt)] env: Env) {
    let (user1, user2, ft1) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(
        vec![user1.account_id(), user2.account_id()],
        vec![ft1.contract_id()],
    )
    .await;

    env.defuse_ft_deposit_to(ft1.contract_id(), 1000, user1.account_id(), None)
        .await
        .unwrap();

    let ft1_token_id = TokenId::from(Nep141TokenId::new(ft1.contract_id().clone()));

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user1.account_id(),
                token_id: &ft1_token_id.to_string()
            })
            .await
            .unwrap()
            .0,
        1000
    );

    let ft_withdraw_intent = FtWithdraw {
        token: ft1.contract_id().clone(),
        receiver_id: user2.account_id().clone(),
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

    let result = env
        .defuse
        .simulate_intents(MultiPayloadArgs {
            signed: vec![ft_withdraw_payload.clone()],
        })
        .await
        .unwrap();

    assert_eq_defuse_event_logs(ft_withdraw_payload.to_event_log(), result.report.logs);
}

#[rstest]
#[tokio::test]
async fn simulate_native_withdraw_intent(#[future(awt)] env: Env) {
    let (user1, user2) = futures::join!(env.create_user(), env.create_user());

    env.initial_ft_storage_deposit(vec![user1.account_id(), user2.account_id()], &[])
        .await;

    let wnear_token_id = TokenId::from(Nep141TokenId::new(env.wnear.contract_id().clone()));

    // Deposit wNEAR to user1's Defuse account
    let wnear_amount = NearToken::from_millinear(100);
    user1
        .near_deposit(env.wnear.contract_id(), wnear_amount)
        .await
        .unwrap();

    user1
        .ft(env.wnear.contract_id())
        .unwrap()
        .transfer_call(
            env.defuse.contract_id(),
            wnear_amount.as_yoctonear(),
            DepositMessage::new(user1.account_id().clone()).to_string(),
        )
        .gas(DEFAULT_GAS)
        .wait_until(Final)
        .await
        .unwrap();

    // Verify wNEAR balance in Defuse
    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user1.account_id(),
                token_id: &wnear_token_id.to_string()
            })
            .await
            .unwrap()
            .0,
        wnear_amount.as_yoctonear()
    );

    let withdraw_amount = NearToken::from_millinear(50);
    let native_withdraw_intent = NativeWithdraw {
        receiver_id: user2.account_id().clone(),
        amount: withdraw_amount,
    };

    let native_withdraw_payload = user1
        .sign_defuse_payload_default(&env.defuse, [native_withdraw_intent.clone()])
        .await
        .unwrap();

    let result = env
        .defuse
        .simulate_intents(MultiPayloadArgs {
            signed: vec![native_withdraw_payload.clone()],
        })
        .await
        .unwrap();

    assert_eq_defuse_event_logs(native_withdraw_payload.to_event_log(), result.report.logs);
}

pub const DUMMY_NFT_URL: &str = "http://example.com/nft/";
pub const DUMMY_NFT_REFERENCE_HASH: [u8; 32] = [13; 32];
pub const DUMMY_NFT_ID: &str = "thisisdummynftid";

#[rstest]
#[tokio::test]
async fn simulate_nft_withdraw_intent(#[future(awt)] env: Env) {
    let (user1, user2) =
        futures::join!(env.create_named_user("nft_issuer_admin"), env.create_user());

    env.transaction(user1.account_id())
        .transfer(NearToken::from_near(100))
        .await
        .unwrap();

    let nft_contract = user1
        .deploy_vanilla_nft_issuer(
            "nft1",
            user1.account_id(),
            &NFTContractMetadata {
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
        .await;

    let _nft = user1
        .mint_nft(
            nft_contract.contract_id(),
            &DUMMY_NFT_ID.to_string(),
            user1.account_id(),
        )
        .await
        .unwrap();

    let nft_token_id: TokenId = Nep171TokenId::new(
        nft_contract.contract_id().to_owned(),
        DUMMY_NFT_ID.to_string(),
    )
    .into();

    user1
        .nft(nft_contract.contract_id())
        .unwrap()
        .transfer_call(
            env.defuse.contract_id(),
            DUMMY_NFT_ID,
            user1.account_id().as_str(),
        )
        .gas(DEFAULT_GAS)
        .wait_until(Final)
        .await
        .unwrap();

    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user1.account_id(),
                token_id: &nft_token_id.to_string()
            })
            .await
            .unwrap()
            .0,
        1
    );

    let nft_withdraw_intent = NftWithdraw {
        token: nft_contract.contract_id().clone(),
        receiver_id: user2.account_id().clone(),
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

    let result = env
        .defuse
        .simulate_intents(MultiPayloadArgs {
            signed: vec![nft_withdraw_payload.clone()],
        })
        .await
        .unwrap();

    assert_eq_defuse_event_logs(nft_withdraw_payload.to_event_log(), result.report.logs);
}

#[rstest]
#[tokio::test]
async fn simulate_mt_withdraw_intent(#[future(awt)] env: Env) {
    let (user1, user2, ft1) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    // Deploy a second Defuse contract which supports NEP-245 operations
    let defuse2 = env
        .deploy_defuse(
            "defuse2",
            DefuseConfig {
                wnear_id: env.wnear.contract_id().clone(),
                fees: FeesConfig {
                    fee: Pips::ZERO,
                    fee_collector: env.account_id().clone(),
                },
                roles: RolesConfig::default(),
            },
            DEFUSE_WASM.clone(),
        )
        .await;

    env.initial_ft_storage_deposit(
        vec![user1.account_id(), user2.account_id()],
        vec![ft1.contract_id()],
    )
    .await;

    // Register user1's public key on defuse2
    user1
        .defuse_add_public_key(defuse2.account_id(), user1.signer().unwrap().public_key())
        .await
        .unwrap();

    let ft1_id = TokenId::from(Nep141TokenId::new(ft1.contract_id().clone()));

    // Step 1: Deposit FT to user1 in the first Defuse contract (stored as MT internally)
    env.defuse_ft_deposit_to(ft1.contract_id(), 1000, user1.account_id(), None)
        .await
        .unwrap();

    // Verify balance in first Defuse contract
    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user1.account_id(),
                token_id: &ft1_id.to_string()
            })
            .await
            .unwrap()
            .0,
        1000
    );

    // Step 2: Transfer tokens from defuse1 to defuse2 using mt_transfer_call
    // This creates MT tokens in defuse2 that can be withdrawn
    user1
        .mt_transfer_call(
            env.defuse.contract_id(),
            defuse2.account_id(),
            ft1_id.to_string(),
            500,
            None,
            user1.account_id().to_string(), // user1 will own these tokens in defuse2
        )
        .await
        .unwrap();

    // Verify tokens are now in defuse2 as NEP-245 tokens
    let nep245_token_id: TokenId =
        Nep245TokenId::new(env.defuse.contract_id().to_owned(), ft1_id.to_string()).into();

    assert_eq!(
        defuse2
            .contract::<Mt>(defuse2.account_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user1.account_id(),
                token_id: &nep245_token_id.to_string()
            })
            .await
            .unwrap()
            .0,
        500
    );

    // Step 3: Create MtWithdraw intent to withdraw MT tokens from defuse2 back to defuse1
    // Now we're simulating on defuse2, withdrawing to defuse1
    let mt_withdraw_intent = MtWithdraw {
        token: env.defuse.contract_id().clone(), // External NEP-245 contract (defuse1)
        receiver_id: user2.account_id().clone(), // Withdraw to user2's account in defuse1
        token_ids: vec![ft1_id.to_string()],     // The FT token ID within defuse1
        amounts: vec![near_sdk::json_types::U128(200)],
        memo: None,
        msg: None,
        storage_deposit: None,
        min_gas: None,
    };

    let defuse2 = env.contract::<Defuse>(defuse2.account_id());
    let mt_withdraw_payload = user1
        .sign_defuse_payload_default(&defuse2, [mt_withdraw_intent.clone()])
        .await
        .unwrap();

    // Simulate the intent on defuse2 (which has the tokens)
    let result = defuse2
        .simulate_intents(MultiPayloadArgs {
            signed: vec![mt_withdraw_payload.clone()],
        })
        .await
        .unwrap();

    assert_eq_defuse_event_logs(mt_withdraw_payload.to_event_log(), result.report.logs);
}

#[rstest]
#[tokio::test]
async fn simulate_storage_deposit_intent(#[future(awt)] env: Env) {
    let (user1, user2, ft1) =
        futures::join!(env.create_user(), env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user1.account_id()], vec![ft1.contract_id()])
        .await;

    let wnear_token_id = TokenId::from(Nep141TokenId::new(env.wnear.contract_id().clone()));

    let wnear_amount = NearToken::from_millinear(100);
    user1
        .near_deposit(env.wnear.contract_id(), wnear_amount)
        .await
        .unwrap();

    user1
        .ft(env.wnear.contract_id())
        .unwrap()
        .transfer_call(
            env.defuse.contract_id(),
            wnear_amount.as_yoctonear(),
            DepositMessage::new(user1.account_id().clone()).to_string(),
        )
        .gas(DEFAULT_GAS)
        .wait_until(Final)
        .await
        .unwrap();

    // Verify wNEAR balance in Defuse
    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user1.account_id(),
                token_id: &wnear_token_id.to_string()
            })
            .await
            .unwrap()
            .0,
        wnear_amount.as_yoctonear()
    );

    let storage_deposit_amount = NearToken::from_millinear(10);
    let storage_deposit_intent = StorageDeposit {
        contract_id: ft1.contract_id().clone(), // Deposit storage on ft1 contract
        deposit_for_account_id: user2.account_id().clone(), // For user2
        amount: storage_deposit_amount,
    };

    let storage_deposit_payload = user1
        .sign_defuse_payload_default(&env.defuse, [storage_deposit_intent.clone()])
        .await
        .unwrap();

    let result = env
        .defuse
        .simulate_intents(MultiPayloadArgs {
            signed: vec![storage_deposit_payload.clone()],
        })
        .await
        .unwrap();

    assert_eq_defuse_event_logs(storage_deposit_payload.to_event_log(), result.report.logs);
}

#[rstest]
#[tokio::test]
async fn simulate_token_diff_intent(#[future(awt)] env: Env) {
    let (user1, user2, ft1, ft2) = futures::join!(
        env.create_user(),
        env.create_user(),
        env.create_token(),
        env.create_token()
    );

    env.initial_ft_storage_deposit(
        vec![user1.account_id(), user2.account_id()],
        vec![ft1.contract_id(), ft2.contract_id()],
    )
    .await;

    let ft1_token_id = TokenId::from(Nep141TokenId::new(ft1.contract_id().clone()));
    let ft2_token_id = TokenId::from(Nep141TokenId::new(ft2.contract_id().clone()));

    // user1 has 100 ft1
    env.defuse_ft_deposit_to(ft1.contract_id(), 100, user1.account_id(), None)
        .await
        .unwrap();

    // user2 has 200 ft2
    env.defuse_ft_deposit_to(ft2.contract_id(), 200, user2.account_id(), None)
        .await
        .unwrap();

    // Verify initial balances
    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user1.account_id(),
                token_id: &ft1_token_id.to_string()
            })
            .await
            .unwrap()
            .0,
        100
    );
    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user2.account_id(),
                token_id: &ft2_token_id.to_string()
            })
            .await
            .unwrap()
            .0,
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

    let user2_payload = user2
        .sign_defuse_payload_default(&env.defuse, [user2_token_diff.clone()])
        .await
        .unwrap();

    let payloads = [user1_payload, user2_payload];
    let result = env
        .defuse
        .simulate_intents(MultiPayloadArgs {
            signed: payloads.to_vec(),
        })
        .await
        .unwrap();

    assert_eq_defuse_event_logs(payloads.to_event_log(), result.report.logs);
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_add_public_key_intent(
    #[notrace]
    #[future(awt)]
    env: Env,
    public_key: PublicKey,
) {
    let user1 = env.create_user().await;

    let new_public_key = public_key;

    let add_public_key_intent = AddPublicKey {
        public_key: new_public_key,
    };

    let add_public_key_payload = user1
        .sign_defuse_payload_default(&env.defuse, [add_public_key_intent])
        .await
        .unwrap();

    let result = env
        .defuse
        .simulate_intents(MultiPayloadArgs {
            signed: vec![add_public_key_payload.clone()],
        })
        .await
        .unwrap();

    assert_eq_defuse_event_logs(add_public_key_payload.to_event_log(), result.report.logs);
}

#[rstest]
#[trace]
#[tokio::test]
async fn simulate_remove_public_key_intent(
    #[notrace]
    #[future(awt)]
    env: Env,
    public_key: PublicKey,
) {
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
    env.defuse_execute_intents(env.defuse.contract_id(), [add_public_key_payload])
        .await
        .unwrap();

    let remove_public_key_intent = RemovePublicKey {
        public_key: new_public_key,
    };

    let remove_public_key_payload = user1
        .sign_defuse_payload_default(&env.defuse, [remove_public_key_intent])
        .await
        .unwrap();

    let result = env
        .defuse
        .simulate_intents(MultiPayloadArgs {
            signed: vec![remove_public_key_payload.clone()],
        })
        .await
        .unwrap();

    assert_eq_defuse_event_logs(remove_public_key_payload.to_event_log(), result.report.logs);
}

#[rstest]
#[tokio::test]
async fn simulate_set_auth_by_predecessor_id_intent(#[future(awt)] env: Env) {
    let user1 = env.create_user().await;

    let set_auth_intent = SetAuthByPredecessorId { enabled: true };

    let set_auth_payload = user1
        .sign_defuse_payload_default(&env.defuse, [set_auth_intent.clone()])
        .await
        .unwrap();

    let result = env
        .defuse
        .simulate_intents(MultiPayloadArgs {
            signed: vec![set_auth_payload.clone()],
        })
        .await
        .unwrap();

    assert_eq_defuse_event_logs(set_auth_payload.to_event_log(), result.report.logs);
}

#[rstest]
#[tokio::test]
async fn simulate_auth_call_intent(#[future(awt)] env: Env) {
    let (user1, ft1) = futures::join!(env.create_user(), env.create_token());

    env.initial_ft_storage_deposit(vec![user1.account_id()], vec![ft1.contract_id()])
        .await;

    let wnear_token_id = TokenId::from(Nep141TokenId::new(env.wnear.contract_id().clone()));

    let wnear_amount = NearToken::from_millinear(100);

    user1
        .near_deposit(env.wnear.contract_id(), wnear_amount)
        .await
        .unwrap();

    user1
        .ft(env.wnear.contract_id())
        .unwrap()
        .transfer_call(
            env.defuse.contract_id(),
            wnear_amount.as_yoctonear(),
            DepositMessage::new(user1.account_id().clone()).to_string(),
        )
        .gas(DEFAULT_GAS)
        .wait_until(Final)
        .await
        .unwrap();

    // Verify wNEAR balance
    assert_eq!(
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id: user1.account_id(),
                token_id: &wnear_token_id.to_string()
            })
            .await
            .unwrap()
            .0,
        wnear_amount.as_yoctonear()
    );

    let auth_call_intent = AuthCall {
        contract_id: ft1.contract_id().clone(), // Call to ft1 contract
        state_init: None,
        msg: "test_message".to_string(),
        attached_deposit: NearToken::from_millinear(10),
        min_gas: None,
    };

    let auth_call_payload = user1
        .sign_defuse_payload_default(&env.defuse, [auth_call_intent])
        .await
        .unwrap();

    let result = env
        .defuse
        .simulate_intents(MultiPayloadArgs {
            signed: vec![auth_call_payload.clone()],
        })
        .await
        .unwrap();

    assert_eq_defuse_event_logs(auth_call_payload.to_event_log(), result.report.logs);
}

#[rstest]
#[tokio::test]
async fn simulate_mint_intent(#[future(awt)] env: Env) {
    let user = env.create_user().await;

    let token_id = "sometoken.near".to_string();
    let memo = "Some memo";
    let amount = 1000;

    let mint_intent = ImtMint {
        tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
        memo: Some(memo.to_string()),
        receiver_id: user.account_id().clone(),
        notification: None,
    };

    let mint_payload = user
        .sign_defuse_payload_default(&env.defuse, [mint_intent.clone()])
        .await
        .unwrap();

    let result = env
        .defuse
        .simulate_intents(MultiPayloadArgs {
            signed: vec![mint_payload.clone()],
        })
        .await
        .unwrap();

    assert_eq_defuse_event_logs(mint_payload.to_event_log(), result.report.logs);
}

#[rstest]
#[tokio::test]
async fn simulate_burn_intent(#[future(awt)] env: Env) {
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
                receiver_id: user.account_id().clone(),
                notification: None,
            }],
        )
        .await
        .unwrap();

    env.defuse_simulate_and_execute_intents(env.defuse.contract_id(), [mint_payload])
        .await
        .unwrap();

    let burn_intent = ImtBurn {
        minter_id: user.account_id().clone(),
        tokens: Amounts::new(std::iter::once((token_id.clone(), amount)).collect()),
        memo: Some(memo.to_string()),
    };

    let burn_payload = user
        .sign_defuse_payload_default(&env.defuse, [burn_intent.clone()])
        .await
        .unwrap();

    let result = env
        .defuse
        .simulate_intents(MultiPayloadArgs {
            signed: vec![burn_payload.clone()],
        })
        .await
        .unwrap();

    assert_eq_defuse_event_logs(burn_payload.to_event_log(), result.report.logs);
}

#[rstest]
#[tokio::test]
async fn simulation_fails_on_used_nonce(#[future(awt)] env: Env) {
    let user = env.create_user().await;

    let payload = user
        .sign_defuse_payload_default(&env.defuse, Vec::<Intent>::new())
        .await
        .unwrap();

    env.defuse_execute_intents(env.defuse.contract_id(), [payload.clone()])
        .await
        .unwrap();

    assert!(
        env.defuse
            .is_nonce_used(IsNonceUsedArgs {
                account_id: user.account_id(),
                nonce: &payload.extract_nonce().unwrap(),
            })
            .await
            .unwrap(),
    );

    let result = env
        .defuse
        .simulate_intents(MultiPayloadArgs {
            signed: vec![payload],
        })
        .await;

    assert!(result.is_err());
}
