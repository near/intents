pub mod nep141;
pub mod nep171;
pub mod nep245;

use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use defuse_core::payload::multi::MultiPayload;
use defuse_near_utils::UnwrapOrPanicError;
use near_account_id::ParseAccountError;
use near_sdk::{AccountId, near, serde_json};
use thiserror::Error as ThisError;

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct DepositMessage {
    pub receiver_id: AccountId,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub execute_intents: Vec<MultiPayload>,

    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    pub refund_if_fails: bool,
}

impl DepositMessage {
    #[must_use]
    #[inline]
    pub const fn new(receiver_id: AccountId) -> Self {
        Self {
            receiver_id,
            execute_intents: Vec::new(),
            refund_if_fails: false,
        }
    }

    #[must_use]
    #[inline]
    pub fn with_execute_intents(mut self, intents: impl IntoIterator<Item = MultiPayload>) -> Self {
        self.execute_intents.extend(intents);
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_refund_if_fails(mut self) -> Self {
        self.refund_if_fails = true;
        self
    }
}

impl Display for DepositMessage {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.execute_intents.is_empty() {
            f.write_str(self.receiver_id.as_str())
        } else {
            f.write_str(&serde_json::to_string(self).unwrap_or_panic_display())
        }
    }
}

impl FromStr for DepositMessage {
    type Err = ParseDepositMessageError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('{') {
            serde_json::from_str(s).map_err(Into::into)
        } else {
            s.parse().map(Self::new).map_err(Into::into)
        }
    }
}

#[derive(Debug, ThisError)]
pub enum ParseDepositMessageError {
    #[error(transparent)]
    Account(#[from] ParseAccountError),
    #[error("JSON: {0}")]
    JSON(#[from] serde_json::Error),
}
