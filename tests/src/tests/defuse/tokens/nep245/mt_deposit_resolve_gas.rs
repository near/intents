use super::binary_search_max;
use crate::tests::defuse::{
    env::{Env, MT_RECEIVER_STUB_WASM},
    tokens::nep245::letter_gen::LetterCombinations,
};
use anyhow::Context;
use defuse::{
    core::intents::tokens::NotifyOnTransfer,
    nep245::{MtBurnEvent, MtEvent, MtMintEvent},
    tokens::{DepositAction, DepositMessage},
};
use defuse_near_utils::REFUND_MEMO;
use defuse_near_utils::TOTAL_LOG_LENGTH_LIMIT;
use defuse_randomness::Rng;
use defuse_sandbox::{SigningAccount, extensions::mt::MtExt};
use defuse_test_utils::random::{gen_random_string, rng};
use multi_token_receiver_stub::MTReceiverMode;
use near_sdk::{AccountId, AsNep297Event, Gas, json_types::U128};
use rstest::rstest;
use std::borrow::Cow;
use std::sync::Arc;

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

async fn make_author_account(mode: TokenIdGenerationMode, env: &Env) -> SigningAccount {
    use near_sdk::NearToken;
    match mode {
        TokenIdGenerationMode::Short => {
            // Use root account directly: 0.test
            env.root().clone()
        }
        TokenIdGenerationMode::Medium => {
            // Use a 64-char named account: {name}.{root_id} = 64 chars total
            const TARGET_LEN: usize = 64;
            let root_id_len = env.root().id().as_str().len();
            // name_len + 1 (dot) + root_id_len = TARGET_LEN
            let name_len = TARGET_LEN - 1 - root_id_len;
            let name = "a".repeat(name_len);
            env.root()
                .generate_subaccount(name, NearToken::from_near(1000))
                .await
                .unwrap()
        }
        TokenIdGenerationMode::Long => {
            // Use implicit account (64 hex chars) for longest account ID
            env.fund_implicit(NearToken::from_near(1000)).await.unwrap()
        }
    }
}

fn make_defuse_token_ids(
    mode: TokenIdGenerationMode,
    author_account: &SigningAccount,
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
            .map(|token_id| format!("nep245:{}:{}", author_account.id(), token_id))
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
        amounts: Cow::Owned(amounts.iter().copied().map(U128).collect()),
        memo: None,
    }]));

    let mt_burn_event = MtEvent::MtBurn(Cow::Owned(vec![MtBurnEvent {
        owner_id: Cow::Borrowed(owner_id),
        authorized_id: None,
        token_ids: Cow::Owned(token_ids.to_vec()),
        amounts: Cow::Owned(amounts.iter().copied().map(U128).collect()),
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
    author_account: SigningAccount,
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
        .mt_on_transfer_raw(
            author_account.id(), // sender_id (who the tokens are being deposited for)
            env.defuse.id(),     // defuse contract receives the deposit
            token_ids.iter().cloned().zip(amounts.clone()),
            serde_json::to_string(&deposit_message).unwrap(),
        )
        .await
        .context("Failed at mt_on_transfer (RPC error)")?;

    let defuse_outcomes: Vec<_> = execution_result
        .outcomes()
        .into_iter()
        .filter(|o| o.executor_id == *env.defuse.id())
        .collect();

    // NOTE:
    // 1st receipt on defuse is the deposit
    // 2nd receipt is resolve notification callback
    // notification callback should panic/fail
    if defuse_outcomes.len() == 2 {
        let resolve_outcome = defuse_outcomes[1].clone();
        let resolve_result = resolve_outcome.into_result();
        assert!(
            resolve_result.is_ok(),
            "CRITICAL: mt_resolve_deposit callback failed for token_count={token_count}! \
            This indicates insufficient gas allocation in the contract. Error: {:?}",
            resolve_result.err()
        );
    }
    // Capture total gas before consuming execution_result
    let total_gas_tgas = execution_result.total_gas_burnt.as_tgas();

    // Extract refund amounts from the final result
    let refund_amounts = execution_result
        .into_result()
        .context("Transaction failed")?
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
    #[values(
        TokenIdGenerationMode::Short,
        TokenIdGenerationMode::Medium,
        TokenIdGenerationMode::Long
    )]
    gen_mode: TokenIdGenerationMode,
    #[values(true, false)] full_coverage: bool,
    rng: impl Rng,
) {
    use defuse_sandbox::tx::FnCallBuilder;
    use near_sdk::NearToken;

    // Skip full_coverage=true when 'long' feature is disabled
    #[cfg(not(feature = "long"))]
    if full_coverage {
        return;
    }
    // Skip full_coverage=false when 'long' feature is enabled
    #[cfg(feature = "long")]
    if !full_coverage {
        return;
    }

    let rng = Arc::new(tokio::sync::Mutex::new(rng));
    let env = Arc::new(Env::new().await);

    env.tx(env.defuse.id())
        .transfer(NearToken::from_near(1000))
        .await
        .unwrap();

    let receiver_stub = env
        .deploy_sub_contract(
            "receiver",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None::<FnCallBuilder>,
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
        let receiver_id = receiver_stub.id().clone();
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
        receiver_stub.id().clone(),
        rng.clone(),
    )
    .await
    .unwrap();

    // When using full coverage mode, run the test for all token counts from 1 to max
    // to ensure the invariant holds for every count, not just the maximum.
    if full_coverage {
        println!("Running exhaustive test for all token counts from 1 to {max_deposited_count}:");
        for token_count in 1..=max_deposited_count {
            run_deposit_resolve_gas_test(
                gen_mode,
                token_count,
                env.clone(),
                author_account.clone(),
                receiver_stub.id().clone(),
                rng.clone(),
            )
            .await
            .unwrap();
        }
    }
}

#[tokio::test]
async fn mt_desposit_resolve_can_handle_large_blob_value_returned_from_notification() {
    use defuse_sandbox::tx::FnCallBuilder;
    use near_sdk::NearToken;

    let env = Arc::new(Env::new().await);
    let amount = 1u128;
    env.tx(env.defuse.id())
        .transfer(NearToken::from_near(1000))
        .await
        .unwrap();

    let receiver_stub = env
        .deploy_sub_contract(
            "receiver",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None::<FnCallBuilder>,
        )
        .await
        .unwrap();

    let author_account = env.fund_implicit(NearToken::from_near(1000)).await.unwrap();
    let deposit_message = DepositMessage {
        receiver_id: receiver_stub.id().clone(),
        action: Some(DepositAction::Notify(
            NotifyOnTransfer::new(
                serde_json::to_string(&MTReceiverMode::ReturnBytes(U128(3 * 1024 * 1024))).unwrap(),
            )
            // NOTE: 300TGas - (10*2+4)
            .with_min_gas(Gas::from_tgas(250)),
        )),
    };

    let execution_result = author_account
        .mt_on_transfer_raw(
            author_account.id(),
            env.defuse.id(),
            [("testtoken1".to_string(), amount)],
            serde_json::to_string(&deposit_message).unwrap(),
        )
        .await
        .expect("Failed at mt_on_transfer (RPC error)");

    let defuse_outcomes: Vec<_> = execution_result
        .outcomes()
        .into_iter()
        .filter(|o| o.executor_id == *env.defuse.id())
        .collect();

    assert!(
        defuse_outcomes.len() >= 2,
        "Expected at least 2 defuse receipts, got {}",
        defuse_outcomes.len()
    );

    let resolve_outcome = defuse_outcomes[1].clone();
    let resolve_result = resolve_outcome.into_result();
    assert!(
        resolve_result.is_ok(),
        "CRITICAL: mt_resolve_deposit callback failed! Error: {:?}",
        resolve_result.err()
    );

    let refund_amounts = execution_result
        .into_result()
        .expect("Transaction failed")
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
