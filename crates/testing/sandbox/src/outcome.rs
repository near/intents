use std::ops::Deref;

use near_kit::rpc::FinalExecutionOutcome;

/// A [`FinalExecutionOutcome`] that passed [`FinalExecutionOutcome::checked`].
///
/// near-kit's `#[must_use]` on [`FinalExecutionOutcome`] also fires after
/// `checked()`. This type drops it, since the check is already done.
#[derive(Debug)]
pub struct CheckedOutcome(FinalExecutionOutcome);

impl CheckedOutcome {
    pub fn into_inner(self) -> FinalExecutionOutcome {
        self.0
    }
}

impl Deref for CheckedOutcome {
    type Target = FinalExecutionOutcome;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Checks both the send error and the execution status in one call:
/// `.await.into_checked()` instead of `.await.unwrap().result().unwrap()`.
pub trait IntoChecked {
    fn into_checked(self) -> anyhow::Result<CheckedOutcome>;
}

impl<E: Into<anyhow::Error>> IntoChecked for Result<FinalExecutionOutcome, E> {
    fn into_checked(self) -> anyhow::Result<CheckedOutcome> {
        Ok(CheckedOutcome(self.map_err(Into::into)?.checked()?))
    }
}
