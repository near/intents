use crate::{
    tests::defuse::{env::Env, tokens::nep245::letter_gen::LetterCombinations},
    utils::{mt::MtExt, test_log::TestLog},
};
use anyhow::Context;
use arbitrary_with::ArbitraryAs;
use defuse::core::{
    crypto,
    token_id::{TokenId, nep245::Nep245TokenId},
};
use defuse_near_utils::arbitrary::ArbitraryAccountId;
use defuse_randomness::Rng;
use defuse_test_utils::random::{gen_random_string, random_bytes, rng};
use near_sdk::NearToken;
use near_workspaces::Account;
use rstest::rstest;
use serde_json::json;
use std::future::Future;
use std::sync::Arc;
use strum::IntoEnumIterator;

/// We generate things based on whether we want everything to be "as long as possible"
/// or "as short as possible", because these affect how much gas is spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display, strum::EnumIter)]
enum GenerationMode {
    ShortestPossible,
    LongestPossible,
}

async fn make_account(mode: GenerationMode, env: &Env) -> Account {
    match mode {
        GenerationMode::ShortestPossible => {
            env.transfer_near(env.user1.id(), NearToken::from_near(1000))
                .await
                .unwrap()
                .unwrap();
            env.user1.clone()
        }
        GenerationMode::LongestPossible => {
            env.transfer_near(env.defuse.id(), NearToken::from_near(1000))
                .await
                .unwrap()
                .unwrap();

            let implicit_account_id = crypto::PublicKey::Ed25519(
                env.user1
                    .secret_key()
                    .public_key()
                    .key_data()
                    .try_into()
                    .unwrap(),
            )
            .to_implicit_account_id();

            env.transfer_near(&implicit_account_id, NearToken::from_near(1000))
                .await
                .unwrap()
                .unwrap();

            let implicit_account = Account::from_secret_key(
                implicit_account_id,
                env.user1.secret_key().clone(),
                env.sandbox().worker(),
            );

            implicit_account
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

/// In this test, we want to ensure that any transfer (with many generation modes) will always succeed and refund.
/// This test is designed to return an error on gracious failure (i.e., when a refund is successful), but to panic
/// if it fails due to failure in refunds.
async fn run_resolve_gas_test(
    gen_mode: GenerationMode,
    token_count: usize,
    env: Arc<Env>,
    author_account: Account,
    rng: Arc<tokio::sync::Mutex<impl Rng>>,
) -> anyhow::Result<()> {
    let mut rng = rng.lock().await;
    let bytes = random_bytes(..1000, &mut rng);
    let mut u = arbitrary::Unstructured::new(&bytes);

    let token_ids = make_token_ids(gen_mode, &mut rng, token_count);
    let amounts = make_amounts(gen_mode, token_count);

    drop(rng);

    let defuse_token_ids = token_ids
        .iter()
        .map(|token_id| {
            TokenId::Nep245(
                Nep245TokenId::new(author_account.id().clone(), token_id.clone()).unwrap(),
            )
            .to_string()
        })
        .collect::<Vec<_>>();

    // Deposit a fictitious token, nep245:user1.test.near:<token-id>, into defuse.
    // This is possible because `mt_on_transfer` creates a token from any contract,
    // where the token id (first part, the contract id part), comes from the caller
    // account id.
    let _on_transfer_test_log: TestLog = author_account
        .call(env.defuse.id(), "mt_on_transfer")
        .args_json(json!({
            "sender_id": env.user1.id(),
            "previous_owner_ids": [env.user1.id()],
            "token_ids": &token_ids,
            "amounts": amounts.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "msg": "",
        }))
        .max_gas()
        .transact()
        .await
        .inspect_err(|e| {
            println!("`mt_on_transfer` (1) failed (expected) for token count `{token_count}`: {e}");
        })
        .context("Failed at mt_on_transfer 1")?
        .into_result()
        .inspect_err(|e| {
            println!("`mt_on_transfer` (2) failed (expected) for token count `{token_count}`: {e}");
        })
        .context("Failed at mt_on_transfer 2")?
        .into();

    // println!(
    //     "({token_count}, {})",
    //     on_transfer_test_log.total_gas_burnt()
    // );

    let non_existent_account = ArbitraryAccountId::arbitrary_as(&mut u).unwrap();

    // We attempt to do a transfer of fictitious token ids from defuse to an arbitrary user.
    // These will fail, but there should be enough gas to do refunds successfully.
    let res = env
        .user1
        .mt_batch_transfer_call(
            env.defuse.id(),
            // Non-existing account id
            &non_existent_account,
            defuse_token_ids.clone(),
            amounts.clone(),
            None::<Vec<_>>,
            None,
            String::new(),
        )
        .await;

    // Assert that a refund happened, since the receiver is non-existent.
    // This is necessary because near-workspaces fails if *any* of the receipts fail within a call.
    // If this doesn't happen, it means that the last call failed at mt_transfer_resolve(). REALLY BAD, BECAUSE NO REFUND HAPPENED!
    assert!(
        env.defuse
            .mt_tokens_for_owner(env.defuse.id(), &non_existent_account, ..=2) // 2 because we only need to check the first N tokens. Good enough.
            .await
            .unwrap()
            .is_empty(),
    );

    let (transferred_amounts, call_test_log) = res
        .inspect_err(|e| {
            println!(
                "`mt_batch_transfer_call` failed (expected) for token count `{token_count}`: {e}"
            );
        })
        .context("Failed at mt_batch_transfer, but refunds succeeded")?;

    println!("{{{token_count}, {}}},", call_test_log.total_gas_burnt());

    // Assert that no transfers happened
    assert_eq!(transferred_amounts, vec![0; token_ids.len()]);

    Ok(())
}

async fn binary_search_max<F, Fut>(low: usize, high: usize, test: F) -> Option<usize>
where
    F: Fn(usize) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let mut lo = low;
    let mut hi = high;
    let mut best = None;

    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        match test(mid).await {
            Ok(()) => {
                best = Some(mid);
                lo = mid + 1; // success -> try higher
            }
            Err(_) => {
                hi = mid - 1; // failure -> try lower
            }
        }
    }

    best
}

#[tokio::test]
#[rstest]
async fn mt_transfer_resolve_gas(rng: impl Rng) {
    let rng = Arc::new(tokio::sync::Mutex::new(rng));
    for gen_mode in GenerationMode::iter() {
        let env = Arc::new(Env::new().await);

        env.transfer_near(env.defuse.id(), NearToken::from_near(1000))
            .await
            .unwrap()
            .unwrap();

        let author_account = make_account(gen_mode, &env).await;

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
