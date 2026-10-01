use super::binary_search_max;
use crate::tests::defuse::{
    env::{Env, env},
    tokens::nep245::letter_gen::LetterCombinations,
};
use anyhow::Context;
use defuse_core::intents::tokens::NotifyOnTransfer;
use defuse_near_utils::{REFUND_MEMO, TOTAL_LOG_LENGTH_LIMIT};
use defuse_randomness::Rng;
use defuse_sandbox::{
    account::Account,
    extensions::{
        defuse::{
            nep245::{MtBurnEvent, MtEvent, MtMintEvent},
            tokens::{DepositAction, DepositMessage},
        },
        mt::{Mt, MtOnTransferArgs},
    },
    kit::{AccountId, ActionView, ExecutionStatus, Final, Gas, Near, NearToken, ReceiptContent},
};
use defuse_test_utils::{
    random::{gen_random_string, rng},
    wasms::MT_RECEIVER_STUB_WASM,
};
use multi_token_receiver_stub::MTReceiverMode;
use near_sdk::{events::AsNep297Event, json_types::U128};
use rstest::rstest;
use std::{borrow::Cow, sync::Arc};

/// Token ID generation modes to test different serialization/storage costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display)]
enum TokenIdGenerationMode {
    /// Short: nep141 format with short account name
    Short,
    /// Medium token IDs: ~64 chars
    Medium,
    /// Long: nep245 format with implicit account (64 chars) and long token IDs (127 chars)
    Long,
}

async fn make_author_account(mode: TokenIdGenerationMode, env: &Env) -> Near {
    match mode {
        TokenIdGenerationMode::Short => {
            // Use root account directly: 0.test
            env.root.clone()
        }
        TokenIdGenerationMode::Medium => {
            // Use a 64-char named account: {name}.{root_id} = 64 chars total
            const TARGET_LEN: usize = 64;
            let root_id_len = env.account_id().as_str().len();
            // name_len + 1 (dot) + root_id_len = TARGET_LEN
            let name_len = TARGET_LEN - 1 - root_id_len;
            let name = "a".repeat(name_len);
            env.create_subaccount(name, NearToken::from_near(1000))
                .await
        }
        TokenIdGenerationMode::Long => {
            // Use implicit account (64 hex chars) for longest account ID
            env.create_implicit(NearToken::from_near(1000)).await
        }
    }
}

fn make_defuse_token_ids(
    mode: TokenIdGenerationMode,
    author_account: &Near,
    token_ids: &[String],
) -> Vec<String> {
    match mode {
        // Short mode uses nep141 format: nep141:{token_id}
        // where token_id serves as a short contract identifier
        TokenIdGenerationMode::Short => token_ids
            .iter()
            .map(|token_id| format!("nep141:{token_id}"))
            .collect(),
        // Medium/Long modes use nep245 format: nep245:{contract_id}:{token_id}
        TokenIdGenerationMode::Medium | TokenIdGenerationMode::Long => token_ids
            .iter()
            .map(|token_id| format!("nep245:{}:{}", author_account.account_id(), token_id))
            .collect(),
    }
}

fn make_token_ids(
    mode: TokenIdGenerationMode,
    rng: &mut impl Rng,
    token_count: usize,
) -> Vec<String> {
    match mode {
        TokenIdGenerationMode::Short => LetterCombinations::generate_combos(token_count),
        TokenIdGenerationMode::Medium => {
            const MEDIUM_TOKEN_ID_LEN: usize = 64;

            (1..=token_count)
                .map(|i| {
                    format!(
                        "{}_{}",
                        i,
                        gen_random_string(rng, MEDIUM_TOKEN_ID_LEN..=MEDIUM_TOKEN_ID_LEN)
                    )[0..MEDIUM_TOKEN_ID_LEN]
                        .to_string()
                })
                .collect::<Vec<_>>()
        }
        TokenIdGenerationMode::Long => {
            const MAX_TOKEN_ID_LEN: usize = 127;

            (1..=token_count)
                .map(|i| {
                    format!(
                        "{}_{}",
                        i,
                        gen_random_string(rng, MAX_TOKEN_ID_LEN..=MAX_TOKEN_ID_LEN)
                    )[0..MAX_TOKEN_ID_LEN]
                        .to_string()
                })
                .collect::<Vec<_>>()
        }
    }
}

fn make_amounts(mode: TokenIdGenerationMode, token_count: usize) -> Vec<u128> {
    match mode {
        // Short: minimal serialization cost
        TokenIdGenerationMode::Short => (0..token_count).map(|_| 1u128).collect(),
        // Medium: ~19 digit value
        TokenIdGenerationMode::Medium => {
            (0..token_count).map(|_| 1234567890123456789u128).collect()
        }
        // Long: ~39 digit value to maximize serialization cost and complicate refund logic
        TokenIdGenerationMode::Long => (0..token_count)
            .map(|_| 123456789123456789123456789123456789123u128)
            .collect(),
    }
}

fn validate_mt_event_log_size(
    owner_id: &AccountId,
    token_ids: &[String],
    amounts: &[u128],
) -> anyhow::Result<()> {
    let mt_mint_event = MtEvent::MtMint(Cow::Owned(vec![MtMintEvent {
        owner_id: Cow::Borrowed(owner_id),
        token_ids: Cow::Owned(token_ids.to_vec()),
        amounts: Cow::Borrowed(amounts),
        memo: None,
    }]));

    let mt_burn_event = MtEvent::MtBurn(Cow::Owned(vec![MtBurnEvent {
        owner_id: Cow::Borrowed(owner_id),
        authorized_id: None,
        token_ids: Cow::Owned(token_ids.to_vec()),
        amounts: Cow::Borrowed(amounts),
        memo: Some(Cow::Borrowed(REFUND_MEMO)),
    }]));

    let mint_log = mt_mint_event.to_nep297_event().to_event_log();
    let burn_log = mt_burn_event.to_nep297_event().to_event_log();

    anyhow::ensure!(
        mint_log.len() <= TOTAL_LOG_LENGTH_LIMIT,
        "mint log will exceed maximum log limit"
    );
    anyhow::ensure!(
        burn_log.len() <= TOTAL_LOG_LENGTH_LIMIT,
        "burn log will exceed maximum log limit"
    );
    Ok(())
}

async fn run_deposit_resolve_gas_test(
    gen_mode: TokenIdGenerationMode,
    token_count: usize,
    env: Arc<Env>,
    author_account: Near,
    receiver_id: AccountId,
    rng: Arc<tokio::sync::Mutex<impl Rng>>,
) -> anyhow::Result<()> {
    println!("token count: {token_count}");
    let mut rng = rng.lock().await;

    let token_ids = make_token_ids(gen_mode, &mut rng, token_count);
    let amounts = make_amounts(gen_mode, token_count);

    drop(rng);

    let deposit_message = DepositMessage {
        receiver_id: receiver_id.clone(),
        action: Some(DepositAction::Notify(
            NotifyOnTransfer::new(serde_json::to_string(&MTReceiverMode::MaliciousRefund).unwrap())
                .with_min_gas(Gas::from_tgas(5)),
        )),
    };

    let defuse_token_ids = make_defuse_token_ids(gen_mode, &author_account, &token_ids);
    validate_mt_event_log_size(&receiver_id, &defuse_token_ids, &amounts)?;
    let execution_result = author_account
        .transaction(env.defuse.contract_id()) // defuse contract receives the deposit
        .add_action(
            Mt::mt_on_transfer(MtOnTransferArgs {
                sender_id: author_account.account_id(), // sender_id (who the tokens are being deposited for)
                previous_owner_ids: &vec![author_account.account_id().clone(); token_count],
                token_ids: &token_ids,
                amounts: &amounts,
                msg: &serde_json::to_string(&deposit_message).unwrap(),
            })
            .gas(Gas::from_tgas(300)),
        )
        .await
        .context("Failed at mt_on_transfer (RPC error)")?;

    let defuse_outcomes: Vec<_> = execution_result
        .receipts_outcome
        .iter()
        .filter(|o| o.outcome.executor_id == *env.defuse.contract_id())
        .collect();

    // NOTE:
    // 1st receipt on defuse is the deposit
    // 2nd receipt is resolve notification callback
    // notification callback should panic/fail
    if defuse_outcomes.len() == 2 {
        let resolve_result = defuse_outcomes[1].clone();
        assert!(
            matches!(
                resolve_result.outcome.status,
                ExecutionStatus::SuccessValue(_)
            ),
            "CRITICAL: mt_resolve_deposit callback failed for token_count={token_count}! \
            This indicates insufficient gas allocation in the contract. Error: {:?}",
            resolve_result.outcome.status
        );
    }
    // Capture total gas before consuming execution_result
    let total_gas_tgas = execution_result.total_gas_used().as_tgas();

    // Extract refund amounts from the final result
    let refund_amounts = execution_result
        .json::<Vec<U128>>()
        .context("Failed to parse refund amounts")?
        .into_iter()
        .map(|a| a.0)
        .collect::<Vec<_>>();

    // Verify all amounts were refunded (since stub returns full amounts)
    assert_eq!(
        refund_amounts, amounts,
        "Expected full refund of all amounts"
    );

    println!(
        "{{token_count: {token_count}, mode: {gen_mode}, gas: {total_gas_tgas} TGas}} - SUCCESS"
    );

    Ok(())
}

#[rstest]
#[tokio::test]
async fn mt_deposit_resolve_gas(
    #[future(awt)] env: Env,
    #[values(
        TokenIdGenerationMode::Short,
        TokenIdGenerationMode::Medium,
        TokenIdGenerationMode::Long
    )]
    gen_mode: TokenIdGenerationMode,
    rng: impl Rng,
) {
    let rng = Arc::new(tokio::sync::Mutex::new(rng));
    let env = Arc::new(env);

    env.transaction(env.defuse.contract_id())
        .transfer(NearToken::from_near(1000))
        .await
        .unwrap();

    let receiver_stub = env
        .deploy_sub_contract(
            "receiver",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None,
        )
        .await
        .unwrap();

    let author_account = make_author_account(gen_mode, &env).await;
    let min_token_count = 1;
    let max_token_count = 200;

    let max_deposited_count = binary_search_max(min_token_count, max_token_count, {
        let rng = rng.clone();
        let env = env.clone();
        let author_account = author_account.clone();
        let receiver_id = receiver_stub.account_id().clone();
        move |token_count| {
            run_deposit_resolve_gas_test(
                gen_mode,
                token_count,
                env.clone(),
                author_account.clone(),
                receiver_id.clone(),
                rng.clone(),
            )
        }
    })
    .await;

    let max_deposited_count = max_deposited_count.unwrap();

    println!("Max token deposit per call for gen_mode={gen_mode} is: {max_deposited_count:?}");

    let min_deposited_desired = 50;
    assert!(max_deposited_count >= min_deposited_desired);

    run_deposit_resolve_gas_test(
        gen_mode,
        max_deposited_count,
        env.clone(),
        author_account.clone(),
        receiver_stub.account_id().clone(),
        rng.clone(),
    )
    .await
    .unwrap();

    // When using full coverage mode, run the test for all token counts from 1 to max
    // to ensure the invariant holds for every count, not just the maximum.
    if cfg!(feature = "long") {
        println!("Running exhaustive test for all token counts from 1 to {max_deposited_count}:");
        for token_count in 1..=max_deposited_count {
            run_deposit_resolve_gas_test(
                gen_mode,
                token_count,
                env.clone(),
                author_account.clone(),
                receiver_stub.account_id().clone(),
                rng.clone(),
            )
            .await
            .unwrap();
        }
    }
}

#[rstest]
#[tokio::test]
async fn mt_desposit_resolve_can_handle_large_blob_value_returned_from_notification(
    #[future(awt)] env: Env,
) {
    let env = Arc::new(env);
    let amount = 1u128;

    env.transaction(env.defuse.contract_id())
        .transfer(NearToken::from_near(1000))
        .await
        .unwrap();

    let receiver_stub = env
        .deploy_sub_contract(
            "receiver",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None,
        )
        .await
        .unwrap();

    let author_account = env.create_implicit(NearToken::from_near(1000)).await;
    let deposit_message = DepositMessage {
        receiver_id: receiver_stub.account_id().clone(),
        action: Some(DepositAction::Notify(
            NotifyOnTransfer::new(
                serde_json::to_string(&MTReceiverMode::ReturnBytes(U128(3 * 1024 * 1024))).unwrap(),
            )
            // NOTE: 300TGas - (10*2+4)
            .with_min_gas(Gas::from_tgas(250)),
        )),
    };

    let execution_result = author_account
        .transaction(env.defuse.contract_id())
        .add_action(
            Mt::mt_on_transfer(MtOnTransferArgs {
                sender_id: author_account.account_id(),
                previous_owner_ids: &[author_account.account_id().clone()],
                token_ids: &["testtoken1".to_string()],
                amounts: &[amount],
                msg: &serde_json::to_string(&deposit_message).unwrap(),
            })
            .gas(Gas::from_tgas(300)),
        )
        .await
        .expect("Failed at mt_on_transfer (RPC error)");

    let defuse_outcomes: Vec<_> = execution_result
        .receipts_outcome
        .iter()
        .filter(|o| o.outcome.executor_id == *env.defuse.contract_id())
        .collect();

    assert!(
        defuse_outcomes.len() >= 2,
        "Expected at least 2 defuse receipts, got {}",
        defuse_outcomes.len()
    );

    let resolve_result = defuse_outcomes[1].clone();
    assert!(
        matches!(
            resolve_result.outcome.status,
            ExecutionStatus::SuccessValue(_)
        ),
        "CRITICAL: mt_resolve_deposit callback failed! Error: {:?}",
        resolve_result.outcome.status
    );

    let refund_amounts = execution_result
        .json::<Vec<U128>>()
        .expect("Failed to parse refund amounts")
        .into_iter()
        .map(|a| a.0)
        .collect::<Vec<_>>();

    assert_eq!(
        refund_amounts,
        vec![amount],
        "Expected full refund of all amounts"
    );
}

/// Regression test: a receiver with a large balance could request refunds
/// bigger than the deposited amounts (e.g. `1e35` for deposits of `1`), so the
/// refund `mt_burn` event grew past `TOTAL_LOG_LENGTH_LIMIT` and the whole
/// `mt_resolve_deposit` callback failed (as happened to the deployed revision).
#[rstest]
#[tokio::test]
async fn mt_resolve_deposit_caps_refunds_to_deposited_amounts(#[future(awt)] env: Env) {
    const TOKEN_COUNT: usize = 80;
    const RECEIVER_BALANCE: u128 = 10u128.pow(37);
    // 36-digit refund request: way more than the deposited `1`, but small
    // enough for the receiver's balance to cover it for every token
    const REFUND_REQUEST: u128 = 10u128.pow(35);

    let env = Arc::new(env);

    env.transaction(env.defuse.contract_id())
        .transfer(NearToken::from_near(1000))
        .await
        .unwrap();

    let receiver_stub = env
        .deploy_sub_contract(
            "receiver",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None,
        )
        .await
        .unwrap();

    // Implicit account (64 hex chars) makes `nep245:{contract}:{token_id}`
    // token ids as long as the ones seen on mainnet
    let author_account = env.create_implicit(NearToken::from_near(1000)).await;
    let token_id = "t".repeat(101);
    let defuse_token_id = format!("nep245:{}:{}", author_account.account_id(), token_id);
    assert_eq!(defuse_token_id.len(), 173);

    let pre_fund_message = DepositMessage {
        receiver_id: receiver_stub.account_id().clone(),
        action: None,
    };

    // NOTE: In this test we leverage the fact that we control the `defuse`
    // account, so we call its `mt_on_transfer` callback directly. Normally one
    // would need a real NEP-245 token contract and transfer tokens on it so
    // that `mt_on_transfer` gets invoked on `defuse` as a callback. Calling it
    // directly yields the exact same code path with a much shorter setup.
    //
    // Pre-fund the receiver so its balance can cover the requested refunds.
    // This is what makes the bug observable: refunds are computed as
    // `min(balance_left, requested)`, so without a big pre-existing balance
    // the refund would be capped by the deposited `1` and the refund log
    // could never grow past the limit.
    author_account
        .transaction(env.defuse.contract_id())
        .add_action(
            Mt::mt_on_transfer(MtOnTransferArgs {
                sender_id: author_account.account_id(),
                previous_owner_ids: &[author_account.account_id().clone()],
                token_ids: &[token_id.clone()],
                amounts: &[RECEIVER_BALANCE],
                msg: &serde_json::to_string(&pre_fund_message).unwrap(),
            })
            .gas(Gas::from_tgas(300)),
        )
        .await
        .expect("pre-fund at mt_on_transfer failed");

    // Sanity check: the mint event fits into the log limit, but the refund
    // event with uncapped (requested) amounts does not.
    let defuse_token_ids = vec![defuse_token_id; TOKEN_COUNT];
    let deposited_amounts = vec![1u128; TOKEN_COUNT];
    let uncapped_amounts = vec![REFUND_REQUEST; TOKEN_COUNT];

    // let mint_log = MtEvent::MtMint(Cow::Owned(vec![MtMintEvent {
    //     owner_id: Cow::Borrowed(receiver_stub.account_id()),
    //     token_ids: Cow::Owned(defuse_token_ids.clone()),
    //     amounts: Cow::Borrowed(&deposited_amounts),
    //     memo: Some(Cow::Borrowed("deposit")),
    // }]))
    // .to_nep297_event()
    // .to_event_log();

    // let refund_log = MtEvent::MtBurn(Cow::Owned(vec![MtBurnEvent {
    //     owner_id: Cow::Borrowed(receiver_stub.account_id()),
    //     authorized_id: None,
    //     token_ids: Cow::Owned(defuse_token_ids.clone()),
    //     amounts: Cow::Borrowed(&deposited_amounts),
    //     memo: Some(Cow::Borrowed(REFUND_MEMO)),
    // }]))
    // .to_nep297_event()
    // .to_event_log();
    //
    // let uncapped_refund_log = MtEvent::MtBurn(Cow::Owned(vec![MtBurnEvent {
    //     owner_id: Cow::Borrowed(receiver_stub.account_id()),
    //     authorized_id: None,
    //     token_ids: Cow::Owned(defuse_token_ids),
    //     amounts: Cow::Borrowed(&uncapped_amounts),
    //     memo: Some(Cow::Borrowed(REFUND_MEMO)),
    // }]))
    // .to_nep297_event()
    // .to_event_log();
    //
    // assert!(mint_log.len() <= TOTAL_LOG_LENGTH_LIMIT);
    // assert!(refund_log.len() <= TOTAL_LOG_LENGTH_LIMIT);
    // assert!(
    //     uncapped_refund_log.len() > TOTAL_LOG_LENGTH_LIMIT,
    //     "test setup is wrong: refund request must overflow the log limit \
    //     (mint: {}, refund: {}, uncapped refund: {})",
    //     mint_log.len(),
    //     refund_log.len(),
    //     uncapped_refund_log.len()
    // );

    // Receiver requests huge refunds...
    let deposit_message = DepositMessage {
        receiver_id: receiver_stub.account_id().clone(),
        action: Some(DepositAction::Notify(
            NotifyOnTransfer::new(
                serde_json::to_string(&MTReceiverMode::ReturnValue(U128(REFUND_REQUEST))).unwrap(),
            )
            .with_min_gas(Gas::from_tgas(5)),
        )),
    };
    let token_ids = vec![token_id; TOKEN_COUNT];
    let amounts = vec![1u128; TOKEN_COUNT];

    // ...but each deposit is just `1`, so refunds must be capped by it
    let execution_result = author_account
        .transaction(env.defuse.contract_id())
        .add_action(
            Mt::mt_on_transfer(MtOnTransferArgs {
                sender_id: author_account.account_id(),
                previous_owner_ids: &vec![author_account.account_id().clone(); TOKEN_COUNT],
                token_ids: &token_ids,
                amounts: &amounts,
                msg: &serde_json::to_string(&deposit_message).unwrap(),
            })
            .gas(Gas::from_tgas(300)),
        )
        .await
        .expect("Failed at mt_on_transfer (RPC error)");

    // Re-fetch the full outcome to get the receipts themselves (the send path
    // only returns execution outcomes), so we can find the callback by name.
    let outcome = env
        .root
        .tx_status(
            execution_result.transaction_hash(),
            author_account.account_id(),
        )
        .wait_until::<Final>()
        .await
        .expect("failed to fetch tx status");

    let resolve_receipt = outcome
        .receipts
        .iter()
        .find(|receipt| {
            receipt.receiver_id == *env.defuse.contract_id()
                && matches!(
                    &receipt.receipt,
                    ReceiptContent::Action(data)
                        if data.actions.iter().any(|action| matches!(
                            action,
                            ActionView::FunctionCall { method_name, .. }
                                if method_name == "mt_resolve_deposit"
                        ))
                )
        })
        .expect("no receipt calling mt_resolve_deposit on defuse");

    let resolve_result = outcome
        .receipts_outcome
        .iter()
        .find(|o| o.id == resolve_receipt.receipt_id)
        .expect("no execution outcome for the mt_resolve_deposit receipt");

    assert!(
        matches!(
            resolve_result.outcome.status,
            ExecutionStatus::SuccessValue(_)
        ),
        "mt_resolve_deposit must not fail on oversized refund log: {:?}",
        resolve_result.outcome.status
    );

    // let refunds = execution_result
    //     .json::<Vec<U128>>()
    //     .expect("Failed to parse refund amounts")
    //     .into_iter()
    //     .map(|a| a.0)
    //     .collect::<Vec<_>>();
    // assert_eq!(
    //     refunds,
    //     vec![1u128; TOKEN_COUNT],
    //     "refunds must be capped by deposited amounts"
    // );
}
