#[cfg(feature = "imt")]
mod imt;
mod nep141;
mod nep171;
mod nep245;

use std::borrow::Cow;

use defuse_sandbox::{
    extensions::defuse::nep245::{MtEvent, MtMintEvent},
    kit::rpc::FinalExecutionOutcome,
};
use near_sdk::{AccountIdRef, events::AsNep297Event};

/// A deposit memo shaped like encrypted recipient
const ENC_MEMO: &str = "AQswVXqfxOkOM1h9osfsETZbgKXK7xQ5XoOozfIXPGGGq9D1Gj9kia7T+B1CZ4yx1vsgRWqPtNn+I0htkrfcASZLcJW63wQpTnOYveIHLFF2m8DlCi9UeZ7D6A0yV3yhxusQNVp/pMnuEzhdgqfM8RY7YIWqz/QZPmOIrdL3HEFmi7DV+h9EaY4=";

/// The `mt_mint` log the Verifier writes for one deposit.
fn mt_mint_log(owner: &AccountIdRef, token_ids: &[String], amounts: &[u128], memo: &str) -> String {
    MtEvent::MtMint(Cow::Owned(vec![MtMintEvent {
        owner_id: Cow::Borrowed(owner),
        token_ids: Cow::Borrowed(token_ids),
        amounts: Cow::Borrowed(amounts),
        memo: Some(Cow::Borrowed(memo)),
    }]))
    .to_nep297_event()
    .to_event_log()
}

/// All `mt_mint` logs of a transaction.
fn mt_mint_logs(outcome: &FinalExecutionOutcome) -> Vec<&str> {
    outcome
        .logs()
        .filter(|log| log.contains(r#""event":"mt_mint""#))
        .collect()
}

/// The status of the first failed receipt that `executor` ran, if any.
fn receipt_failure(outcome: &FinalExecutionOutcome, executor: &AccountIdRef) -> Option<String> {
    outcome
        .receipts_outcome
        .iter()
        .find(|o| {
            o.outcome.executor_id.as_str() == executor.as_str() && o.outcome.status.is_failure()
        })
        .map(|o| format!("{:?}", o.outcome.status))
}
