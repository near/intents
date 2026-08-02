#[cfg(feature = "near-kit")]
pub mod client;
pub mod contract;
mod message;
#[cfg(feature = "resolver")]
pub mod resolver;

pub use self::message::*;

use near_account_id::AccountId;

#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AuthorizationResolution {
    // TODO: rename to "authorized"?
    pub output: String,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub pending: Vec<PendingAuthorization>,
}

impl AuthorizationResolution {
    #[inline]
    pub fn new(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            pending: Vec::new(),
        }
    }

    #[inline]
    pub fn pending(
        mut self,
        account_id: impl Into<AccountId>,
        input: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        self.pending.push(PendingAuthorization {
            account_id: account_id.into(),
            input: input.into(),
            output: output.into(),
        });
        self
    }

    #[inline]
    pub const fn is_terminate(&self) -> bool {
        self.pending.is_empty()
    }
}

impl Extend<PendingAuthorization> for AuthorizationResolution {
    #[inline]
    fn extend<T: IntoIterator<Item = PendingAuthorization>>(&mut self, iter: T) {
        self.pending.extend(iter);
    }
}

#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PendingAuthorization {
    // TODO: returning self should be not allowed
    pub account_id: AccountId,
    // TODO: method like in JSON-RPC?
    pub input: String,
    // TODO: can we avoid fixing the output?
    // callback pattern: ft::w_resolve_auth("tell me how many tokens a user has and call me back with the number + this string")
    pub output: String,
}
