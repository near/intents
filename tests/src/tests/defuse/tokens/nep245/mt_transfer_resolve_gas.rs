use super::binary_search_max;
use crate::tests::defuse::{
    env::{Env, MT_RECEIVER_STUB_WASM},
    tokens::nep245::letter_gen::LetterCombinations,
};
use anyhow::Context;
use arbitrary::Arbitrary;
use defuse::{
    core::token_id::{TokenId, nep245::Nep245TokenId},
    nep245::{MtEvent, MtTransferEvent},
};
use defuse_randomness::Rng;
use defuse_sandbox::{
    SigningAccount,
    extensions::mt::{MtExt, MtViewExt},
    tx::FnCallBuilder,
};
use defuse_test_utils::random::{gen_random_string, random_bytes, rng};
use multi_token_receiver_stub::MTReceiverMode;
use near_sdk::{AccountId, AsNep297Event, NearToken, json_types::U128};
use rstest::rstest;
use std::borrow::Cow;
use std::sync::Arc;
use strum::IntoEnumIterator;

const TOTAL_LOG_LENGTH_LIMIT: usize = 16384;

/// We generate things based on whether we want everything to be "as long as possible"
/// or "as short as possible", because these affect how much gas is spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display, strum::EnumIter)]
enum GenerationMode {
    ShortestPossible,
    LongestPossible,
}

async fn make_account(mode: GenerationMode, env: &Env, user: &SigningAccount) -> SigningAccount {
    match mode {
        GenerationMode::ShortestPossible => {
            env.tx(user.id())
                .transfer(NearToken::from_near(1000))
                .await
                .unwrap();
            user.clone()
        }
        GenerationMode::LongestPossible => {
            env.tx(env.defuse.id())
                .transfer(NearToken::from_near(1000))
                .await
                .unwrap();

            env.fund_implicit(NearToken::from_near(1000)).await.unwrap()
        }
    }
}

fn make_token_ids(mode: GenerationMode, rng: &mut impl Rng, token_count: usize) -> Vec<String> {
    match mode {
        GenerationMode::ShortestPossible => LetterCombinations::generate_combos(token_count),
        GenerationMode::LongestPossible => {
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
        memo: Some(Cow::Borrowed("refund")),
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
    user_account: SigningAccount,
    author_account: SigningAccount,
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
                author_account.id().clone(),
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
            user_account.id(),
            env.defuse.id(),
            token_ids.iter().zip(amounts.clone()),
            "",
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
        user_account.id(),
        &non_existent_account,
        &defuse_token_ids,
        &amounts,
    )?;

    println!("Non-existent account: {non_existent_account}");

    assert!(
        env.defuse
            .mt_tokens_for_owner(&non_existent_account, ..=2) // 2 because we only need to check the first N tokens. Good enough.
            .await
            .unwrap()
            .is_empty(),
    );

    println!("max transfer amount: {}", amounts.iter().max().unwrap());

    // We attempt to do a transfer of fictitious token ids from defuse to an arbitrary user.
    // These will fail, but there should be enough gas to do refunds successfully.
    let res = user_account
        .mt_batch_transfer_call(
            env.defuse.id(),
            // Non-existing account id
            &non_existent_account,
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
        .unwrap();

    // Assert that a refund happened, since the receiver is non-existent.
    // This is necessary because near-workspaces fails if *any* of the receipts fail within a call.
    // If this doesn't happen, it means that the last call failed at mt_transfer_resolve(). REALLY BAD, BECAUSE NO REFUND HAPPENED!
    assert!(
        env.defuse
            .mt_tokens_for_owner(&non_existent_account, ..=2) // 2 because we only need to check the first N tokens. Good enough.
            .await
            .unwrap()
            .is_empty(),
    );

    println!("{{{token_count}, {}}},", res.total_gas_burnt);

    let transferred_amounts = res
        .clone()
        .json::<Vec<U128>>()
        .map(|refunds| refunds.into_iter().map(|a| a.0).collect::<Vec<u128>>())
        .context("Failed to parse refunds from mt_batch_transfer_call")?;

    let longest_emited_log = res.logs().iter().map(|s| s.len()).max().unwrap();

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
async fn mt_transfer_resolve_gas(rng: impl Rng) {
    let rng = Arc::new(tokio::sync::Mutex::new(rng));
    for gen_mode in GenerationMode::iter() {
        let env = Arc::new(Env::new().await);

        let user = env.create_user().await;

        env.tx(env.defuse.id())
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
            "Max token transfer per call for generation mode {gen_mode} is: {max_transferred_count:?}"
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

#[tokio::test]
async fn mt_batch_transfer_call_rejects_transfer_when_refund_log_exceeds_limit() {
    let env = Env::new().await;
    let user = env.create_named_user("user").await;

    env.tx(env.defuse.id())
        .transfer(NearToken::from_near(1000))
        .await
        .unwrap();

    let author_account = env.fund_implicit(NearToken::from_near(1000)).await.unwrap();

    let receiver_stub = env
        .deploy_sub_contract(
            "receiver",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None::<FnCallBuilder>,
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
                author_account.id().clone(),
                token_id.clone(),
            ))
            .to_string()
        })
        .collect();

    let (transfer_log_size, refund_log_size) =
        calculate_log_sizes(user.id(), receiver_stub.id(), &defuse_token_ids, &amounts);

    assert!(transfer_log_size <= TOTAL_LOG_LENGTH_LIMIT,);
    assert!(refund_log_size > TOTAL_LOG_LENGTH_LIMIT,);

    author_account
        .mt_on_transfer(
            user.id(),
            env.defuse.id(),
            token_ids.iter().cloned().zip(amounts.clone()),
            "",
        )
        .await
        .unwrap();

    let balance_before = env
        .defuse
        .mt_balance_of(user.id(), &defuse_token_ids[0])
        .await
        .unwrap();

    let result = user
        .mt_batch_transfer_call(
            env.defuse.id(),
            receiver_stub.id(),
            defuse_token_ids.clone(),
            amounts.clone(),
            None,
            serde_json::to_string(&MTReceiverMode::RefundAll).unwrap(),
        )
        .await
        .unwrap();

    assert!(
        result.is_failure(),
        "transfer should fail early due to refund log size limit"
    );

    let result_str = format!("{result:?}");
    assert!(
        result_str.contains("Event log is too long"),
        "expected error about refund log limit, got: {result_str}"
    );

    let balance_after = env
        .defuse
        .mt_balance_of(user.id(), &defuse_token_ids[0])
        .await
        .unwrap();

    assert_eq!(balance_after, balance_before,);
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
        memo: Some(Cow::Borrowed("refund")), // Refund has "refund" memo
    }]));

    let transfer_log_size = transfer_event.to_nep297_event().to_event_log().len();
    let refund_log_size = refund_event.to_nep297_event().to_event_log().len();

    (transfer_log_size, refund_log_size)
}
