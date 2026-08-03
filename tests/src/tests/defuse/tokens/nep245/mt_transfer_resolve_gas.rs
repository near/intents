use super::binary_search_max;
use crate::tests::defuse::env::{Env, env};
use crate::tests::defuse::tokens::nep245::letter_gen::LetterCombinations;
use anyhow::Context;
use arbitrary::Arbitrary;
use defuse_near_utils::REFUND_MEMO;
use defuse_randomness::Rng;
use defuse_sandbox::{
    account::Account,
    extensions::{
        defuse::{
            core::{
                token_id::{TokenId, nep245::Nep245TokenId},
                tokens::MAX_TOKEN_ID_LEN,
            },
            nep245::{MtEvent, MtTransferEvent},
        },
        mt::MtOnTransferArgs,
        mt::{Mt, MtBalanceOfArgs, MtBatchTransferCallArgs, MtExt},
    },
    kit::{
        AccountId, ActionError, ActionErrorKind, ExecutionStatus, Final, FunctionCallError, Gas,
        Near, NearToken,
    },
};
use defuse_test_utils::{
    random::{gen_random_string, random_bytes, rng},
    wasms::MT_RECEIVER_STUB_WASM,
};
use multi_token_receiver_stub::MTReceiverMode;
use near_sdk_core::{events::AsNep297Event, json_types::U128};
use rstest::rstest;
use std::{borrow::Cow, sync::Arc};
use strum::IntoEnumIterator;

const TOTAL_LOG_LENGTH_LIMIT: usize = 16384;

/// We generate things based on whether we want everything to be "as long as possible"
/// or "as short as possible", because these affect how much gas is spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter)]
enum GenerationMode {
    ShortestPossible,
    LongestPossible,
}

async fn make_account(mode: GenerationMode, env: &Env, user: &Near) -> Near {
    match mode {
        GenerationMode::ShortestPossible => {
            env.transaction(user.account_id())
                .transfer(NearToken::from_near(1000))
                .await
                .unwrap()
                .result()
                .unwrap();
            user.clone()
        }
        GenerationMode::LongestPossible => {
            env.transaction(env.defuse.contract_id())
                .transfer(NearToken::from_near(1000))
                .await
                .unwrap()
                .result()
                .unwrap();

            env.create_implicit(NearToken::from_near(1000)).await
        }
    }
}

fn make_token_ids(mode: GenerationMode, rng: &mut impl Rng, token_count: usize) -> Vec<String> {
    match mode {
        GenerationMode::ShortestPossible => LetterCombinations::generate_combos(token_count),
        GenerationMode::LongestPossible => (1..=token_count)
            .map(|i| {
                format!(
                    "{}_{}",
                    i,
                    gen_random_string(rng, MAX_TOKEN_ID_LEN..=MAX_TOKEN_ID_LEN)
                )[0..MAX_TOKEN_ID_LEN]
                    .to_string()
            })
            .collect::<Vec<_>>(),
    }
}

fn make_amounts(mode: GenerationMode, token_count: usize) -> Vec<u128> {
    match mode {
        GenerationMode::ShortestPossible => (0..token_count).map(|_| 1).collect(),
        GenerationMode::LongestPossible => (0..token_count).map(|_| u128::MAX).collect(),
    }
}

fn validate_mt_batch_transfer_log_size(
    sender_id: &AccountId,
    receiver_id: &AccountId,
    token_ids: &[String],
    amounts: &[u128],
) -> anyhow::Result<usize> {
    let mt_transfer_event = MtEvent::MtTransfer(Cow::Owned(vec![MtTransferEvent {
        authorized_id: None,
        old_owner_id: Cow::Borrowed(receiver_id),
        new_owner_id: Cow::Borrowed(sender_id),
        token_ids: Cow::Owned(token_ids.to_vec()),
        amounts: Cow::Owned(amounts.iter().copied().map(U128).collect()),
        memo: Some(Cow::Borrowed(REFUND_MEMO)),
    }]));

    let longest_transfer_log = mt_transfer_event.to_nep297_event().to_event_log();

    anyhow::ensure!(
        longest_transfer_log.len() <= TOTAL_LOG_LENGTH_LIMIT,
        "transfer log will exceed maximum log limit"
    );

    Ok(longest_transfer_log.len())
}

/// In this test, we want to ensure that any transfer (with many generation modes) will always succeed and refund.
/// This test is designed to return an error on gracious failure (i.e., when a refund is successful), but to panic
/// if it fails due to failure in refunds.
async fn run_resolve_gas_test(
    gen_mode: GenerationMode,
    token_count: usize,
    env: Arc<Env>,
    user_account: Near,
    author_account: Near,
    rng: Arc<tokio::sync::Mutex<impl Rng>>,
) -> anyhow::Result<()> {
    println!("token count: {token_count}");
    let mut rng = rng.lock().await;
    let bytes = random_bytes(..1000, &mut rng);
    let mut u = arbitrary::Unstructured::new(&bytes);

    let token_ids = make_token_ids(gen_mode, &mut rng, token_count);
    let amounts = make_amounts(gen_mode, token_count);

    drop(rng);

    let defuse_token_ids = token_ids
        .iter()
        .map(|token_id| {
            TokenId::Nep245(Nep245TokenId::new(
                author_account.account_id().clone(),
                token_id.clone(),
            ))
            .to_string()
        })
        .collect::<Vec<_>>();

    // Deposit a fictitious token, nep245:user.test.near:<token-id>, into defuse.
    // This is possible because `mt_on_transfer` creates a token from any contract,
    // where the token id (first part, the contract id part), comes from the caller
    // account id.

    author_account
        .mt_on_transfer(
            env.defuse.contract_id(),
            MtOnTransferArgs {
                sender_id: user_account.account_id(),
                previous_owner_ids: &vec![author_account.account_id().clone(); token_ids.len()],
                token_ids: &token_ids,
                amounts: &amounts,
                msg: "",
            },
        )
        .await
        .inspect_err(|e| {
            println!("`mt_on_transfer` failed (expected) for token count `{token_count}`: {e}");
        })
        .context("Failed at mt_on_transfer")?;

    let non_existent_account = AccountId::arbitrary(&mut u).unwrap();

    // NOTE: `mt_on_transfer` emits an `MtMint` event, but `mt_batch_transfer_call` emits `mt_transfer`
    // events that serialize more fields. These transfer logs approach the hard log-size limit, so
    // we pre-calculate the worst-case payload to fail fast if the limit would be exceeded.
    let expected_transfer_log = validate_mt_batch_transfer_log_size(
        user_account.account_id(),
        &non_existent_account,
        &defuse_token_ids,
        &amounts,
    )?;

    println!("Non-existent account: {non_existent_account}");

    assert!(
        env.mt_tokens_for_owner(env.defuse.contract_id(), &non_existent_account, ..=2)
            .await
            .unwrap()
            .is_empty(),
    );

    println!("max transfer amount: {}", amounts.iter().max().unwrap());

    // We attempt to do a transfer of fictitious token ids from defuse to an arbitrary user.
    // These will fail, but there should be enough gas to do refunds successfully.
    let (res, transferred_amounts) = user_account
        .mt_batch_transfer_call(
            env.defuse.contract_id(),
            // Non-existing account id
            non_existent_account.clone(),
            defuse_token_ids.clone(),
            amounts.clone(),
            None,
            String::new(),
        )
        .await
        .inspect_err(|e| {
            println!(
                "`mt_batch_transfer_call` failed (expected) for token count `{token_count}`: {e}"
            );
        })
        .context("Failed at mt_batch_transfer_call")?;

    // Assert that a refund happened, since the receiver is non-existent.
    // This is necessary because near-workspaces fails if *any* of the receipts fail within a call.
    // If this doesn't happen, it means that the last call failed at mt_transfer_resolve(). REALLY BAD, BECAUSE NO REFUND HAPPENED!
    assert!(
        env.mt_tokens_for_owner(env.defuse.contract_id(), &non_existent_account, ..=2)
            .await
            .unwrap()
            .is_empty(),
    );

    let longest_emited_log = res.logs().iter().map(String::len).max().unwrap();

    assert_eq!(
        longest_emited_log, expected_transfer_log,
        "transfer log does not match expected transfer log"
    );

    // Assert that no transfers happened
    assert_eq!(transferred_amounts, vec![0; token_ids.len()]);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn mt_transfer_resolve_gas(#[future(awt)] env: Env, rng: impl Rng) {
    let rng = Arc::new(tokio::sync::Mutex::new(rng));
    let env = Arc::new(env);

    for gen_mode in GenerationMode::iter() {
        let user = env.create_user().await;

        env.transaction(env.defuse.contract_id())
            .transfer(NearToken::from_near(1000))
            .await
            .unwrap();

        let author_account = make_account(gen_mode, &env, &user).await;

        let min_token_count = 1;
        let max_token_count = 200;

        let max_transferred_count = binary_search_max(min_token_count, max_token_count, {
            let rng = rng.clone();
            let env = env.clone();
            let author_account = author_account.clone();
            move |token_count| {
                run_resolve_gas_test(
                    gen_mode,
                    token_count,
                    env.clone(),
                    user.clone(),
                    author_account.clone(),
                    rng.clone(),
                )
            }
        })
        .await;

        let max_transferred_count = max_transferred_count.unwrap();

        println!(
            "Max token transfer per call for generation mode {gen_mode:?} is: {max_transferred_count:?}"
        );

        // If the max number of transferred tokens is less than this value, panic.
        let min_transferred_desired = 50;
        assert!(max_transferred_count >= min_transferred_desired);
    }
}

#[tokio::test]
async fn binary_search() {
    let max = 100;
    // Test all possible values for binary search
    for limit in 0..max {
        let test = move |x| async move {
            if x <= limit {
                Ok(())
            } else {
                Err(anyhow::anyhow!(">limit"))
            }
        };
        assert_eq!(binary_search_max(0, max, test).await, Some(limit));
    }
}

#[rstest]
#[tokio::test]
async fn mt_batch_transfer_call_rejects_transfer_when_refund_log_exceeds_limit(
    #[future(awt)] env: Env,
) {
    let user = env.create_named_user("u").await;

    env.transaction(env.defuse.contract_id())
        .transfer(NearToken::from_near(1000))
        .await
        .unwrap();

    let author_account = env.create_implicit(NearToken::from_near(1000)).await;

    let receiver_stub = env
        .deploy_sub_contract(
            "r",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None,
        )
        .await
        .unwrap();

    let gen_max_len_token_id = |i: usize| format!("{i}{}", "a".repeat(127 - i.to_string().len()));
    let token_ids: Vec<String> = (1..=65)
        .map(gen_max_len_token_id)
        .chain([
            "1thiswilltriggertoolonglogerrorthiswilltriggertoolonglo".to_string(),
            "2thiswilltriggertoolonglogerrorthiswilltriggertoolonglo".to_string(),
        ])
        .collect();

    let amounts: Vec<u128> = vec![u128::MAX; token_ids.len()];
    let defuse_token_ids: Vec<String> = token_ids
        .iter()
        .map(|token_id| {
            TokenId::Nep245(Nep245TokenId::new(
                author_account.account_id().clone(),
                token_id.clone(),
            ))
            .to_string()
        })
        .collect();

    let (transfer_log_size, refund_log_size) = calculate_log_sizes(
        user.account_id(),
        receiver_stub.account_id(),
        &defuse_token_ids,
        &amounts,
    );

    assert!(transfer_log_size <= TOTAL_LOG_LENGTH_LIMIT,);
    assert!(refund_log_size > TOTAL_LOG_LENGTH_LIMIT,);

    author_account
        .mt_on_transfer(
            env.defuse.contract_id(),
            MtOnTransferArgs {
                sender_id: user.account_id(),
                previous_owner_ids: &vec![author_account.account_id().clone(); token_ids.len()],
                token_ids: &token_ids,
                amounts: &amounts,
                msg: "",
            },
        )
        .await
        .unwrap();

    let balance_before = env
        .contract::<Mt>(env.defuse.contract_id())
        .mt_balance_of(MtBalanceOfArgs {
            account_id: user.account_id(),
            token_id: &defuse_token_ids[0],
        })
        .await
        .unwrap();

    let result = user
        .mt_batch_transfer_call(
            env.defuse.contract_id(),
            receiver_stub.account_id(),
            defuse_token_ids.clone(),
            amounts.clone(),
            None,
            serde_json::to_string(&MTReceiverMode::RefundAll).unwrap(),
        )
        .await;

    assert!(
        result.is_err(),
        "transfer should fail early due to refund log size limit"
    );

    let result_str = format!("{result:?}");
    assert!(
        result_str.contains("refund event log would be too long"),
        "expected error about refund log limit, got: {result_str}"
    );

    let balance_after = env
        .contract::<Mt>(env.defuse.contract_id())
        .mt_balance_of(MtBalanceOfArgs {
            account_id: user.account_id(),
            token_id: &defuse_token_ids[0],
        })
        .await
        .unwrap();

    assert_eq!(balance_after, balance_before,);
}

const REPRO_TOKEN_COUNT: usize = 66;
const REPRO_FIRST_TOKEN_ID_LEN: usize = MAX_TOKEN_ID_LEN - 8;
const REPRO_TOKEN_ID_PREFIX_LEN: usize = 2;

/// `mt_resolve_transfer()` refunds by re-emitting the transfer with `memo: "refund"`, which
/// makes the refund log exactly `,"memo":"refund"` (16 bytes) longer than the forward one.
/// `check_refund()` must therefore reject any no-memo transfer longer than
/// `TOTAL_LOG_LENGTH_LIMIT - 16`.
///
/// This pins the boundary: the shortest forward log whose refund overflows. Accepting it
/// commits the transfer in the first receipt and then aborts `mt_resolve_transfer()` on
/// `env::log_str()`, leaving the tokens on the receiver with no way to recover them
/// (`mt_resolve_transfer` is `#[private]` and fires once).
///
/// Every account here is implicit, i.e. exactly [`AccountId::MAX_LEN`] bytes, so the log
/// sizes are fixed rather than depending on the process-global counter that
/// `defuse_sandbox::root` bakes into the test root's name.
#[rstest]
#[tokio::test]
async fn mt_batch_transfer_call_refunds_at_exact_log_limit_boundary(#[future(awt)] env: Env) {
    env.transaction(env.defuse.contract_id())
        .transfer(NearToken::from_near(1000))
        .await
        .unwrap();

    let author_account = env.create_implicit(NearToken::from_near(1000)).await;
    let user = env.create_implicit(NearToken::from_near(1000)).await;
    let receiver_stub = env.create_implicit(NearToken::from_near(100)).await;
    receiver_stub
        .deploy(MT_RECEIVER_STUB_WASM.to_vec())
        .wait_until::<Final>()
        .await
        .unwrap()
        .result()
        .unwrap();

    let token_ids: Vec<String> = std::iter::once(REPRO_FIRST_TOKEN_ID_LEN)
        .chain(std::iter::repeat_n(MAX_TOKEN_ID_LEN, REPRO_TOKEN_COUNT - 1))
        .enumerate()
        .map(|(i, len)| {
            // Index-prefixed to keep them unique; all indices fit the fixed-width prefix.
            format!("{i:02}{}", "a".repeat(len - REPRO_TOKEN_ID_PREFIX_LEN))
        })
        .collect();
    let amounts = vec![u128::MAX; REPRO_TOKEN_COUNT];
    let defuse_token_ids: Vec<String> = token_ids
        .iter()
        .map(|token_id| {
            TokenId::Nep245(Nep245TokenId::new(
                author_account.account_id().clone(),
                token_id.clone(),
            ))
            .to_string()
        })
        .collect();

    author_account
        .mt_on_transfer(
            env.defuse.contract_id(),
            MtOnTransferArgs {
                sender_id: user.account_id(),
                previous_owner_ids: &vec![author_account.account_id().clone(); token_ids.len()],
                token_ids: &token_ids,
                amounts: &amounts,
                msg: "",
            },
        )
        .await
        .unwrap();

    let balance_of = async |account_id| {
        env.contract::<Mt>(env.defuse.contract_id())
            .mt_balance_of(MtBalanceOfArgs {
                account_id,
                token_id: &defuse_token_ids[0],
            })
            .await
            .unwrap()
    };
    let balance_before = balance_of(user.account_id()).await;

    // NOTE: called as a raw transaction rather than through `MtExt::mt_batch_transfer_call`,
    // since that helper drops the outcome when any receipt in the chain fails.
    let execution_result = user
        .transaction(env.defuse.contract_id())
        .add_action(
            Mt::mt_batch_transfer_call(MtBatchTransferCallArgs {
                receiver_id: receiver_stub.account_id(),
                token_ids: &defuse_token_ids,
                amounts: &amounts,
                approvals: None,
                memo: None,
                msg: &serde_json::to_string(&MTReceiverMode::RefundAll).unwrap(),
            })
            .deposit(NearToken::from_yoctonear(1))
            .gas(Gas::from_tgas(300)),
        )
        .wait_until::<Final>()
        .await
        .unwrap();

    // 1st receipt on defuse is `mt_batch_transfer_call` itself; 2nd is the `mt_resolve_transfer` callback.
    let defuse_outcomes: Vec<_> = execution_result
        .receipts_outcome
        .iter()
        .filter(|o| o.outcome.executor_id == *env.defuse.contract_id())
        .collect();
    assert_eq!(defuse_outcomes.len(), 2);
    assert!(
        matches!(
            &defuse_outcomes[1].outcome.status,
            ExecutionStatus::Failure(ActionError {
                kind: ActionErrorKind::FunctionCallError(FunctionCallError::ExecutionError(msg)),
                ..
            }) if msg.contains("length of a log message") && msg.contains("exceeds the limit")
        ),
        "expected mt_resolve_transfer to fail specifically due to the refund log exceeding \
        the total log length limit, got: {:?}",
        defuse_outcomes[1].outcome.status
    );

    assert_eq!(
        balance_of(user.account_id()).await,
        balance_before,
        "sender was not made whole"
    );
}

/// Calculate log sizes for transfer (no memo) and refund (with "refund" memo).
fn calculate_log_sizes(
    sender_id: &AccountId,
    receiver_id: &AccountId,
    token_ids: &[String],
    amounts: &[u128],
) -> (usize, usize) {
    let transfer_event = MtEvent::MtTransfer(Cow::Owned(vec![MtTransferEvent {
        authorized_id: None,
        old_owner_id: Cow::Borrowed(sender_id),
        new_owner_id: Cow::Borrowed(receiver_id),
        token_ids: Cow::Owned(token_ids.to_vec()),
        amounts: Cow::Owned(amounts.iter().copied().map(U128).collect()),
        memo: None, // Transfer has no memo
    }]));

    let refund_event = MtEvent::MtTransfer(Cow::Owned(vec![MtTransferEvent {
        authorized_id: None,
        old_owner_id: Cow::Borrowed(receiver_id),
        new_owner_id: Cow::Borrowed(sender_id),
        token_ids: Cow::Owned(token_ids.to_vec()),
        amounts: Cow::Owned(amounts.iter().copied().map(U128).collect()),
        memo: Some(Cow::Borrowed(REFUND_MEMO)), // Refund has "refund" memo
    }]));

    let transfer_log_size = transfer_event.to_nep297_event().to_event_log().len();
    let refund_log_size = refund_event.to_nep297_event().to_event_log().len();

    (transfer_log_size, refund_log_size)
}
