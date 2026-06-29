pub mod actions;

pub use near_account_id::{self as account_id, AccountId, AccountIdRef};
pub use near_gas::NearGas as Gas;
// TODO
pub use near_global_contracts::StateInit;
pub use near_token::NearToken;

use self::actions::{FunctionCall, NearAction, StateInitAction, Transfer};

/// A single outgoing promise
#[must_use = "promises do nothing unless you `.build()` them"]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NearPromise {
    /// Receiver of the receipt to be created.
    ///
    /// NOTE: self-calls are prohibited.
    pub receiver_id: AccountId,

    /// Receiver for refunds of failed or unused NEAR deposits.
    /// By default, it's the caller contract itself.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub refund_to: Option<AccountId>,

    /// Actions to execute within a single promise.
    ///
    /// TODO: what empty list will result in?
    pub actions: Vec<NearAction>,
    // TODO: future compatibility with other fields? versioned?
}

impl NearPromise {
    /// Create a new promise to given `receiver_id`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_near_promise::{AccountIdRef, NearPromise};
    /// # let account_id = AccountIdRef::new_or_panic("ft.near");
    /// let p = NearPromise::new(account_id);
    ///
    /// assert_eq!(p.receiver_id, account_id);
    /// ```
    #[inline]
    pub fn new(receiver_id: impl Into<AccountId>) -> Self {
        Self {
            receiver_id: receiver_id.into(),
            refund_to: None,
            actions: Vec::new(),
        }
    }

    /// Set an account where all failed/unused deposits should be refunded
    /// instead of the wallet-contract itself.
    #[inline]
    pub fn refund_to(mut self, account_id: impl Into<AccountId>) -> Self {
        self.refund_to = Some(account_id.into());
        self
    }

    #[inline]
    pub fn transfer(self, amount: NearToken) -> Self {
        self.add_action(Transfer { amount })
    }

    // TODO
    #[inline]
    pub fn state_init(self, state_init: impl Into<Vec<u8>>, deposit: NearToken) -> Self {
        self.add_action(StateInitAction::new(state_init).deposit(deposit))
    }

    /// Add `FunctionCall` action to this promise
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use serde_json::json;
    /// # use defuse_near_promise::{
    /// #    AccountIdRef, Gas, NearPromise, NearToken,
    /// #    actions::FunctionCall,
    /// # };
    /// # let contract_id = AccountIdRef::new_or_panic("ft.near");
    /// let _ = NearPromise::new(contract_id)
    ///     .function_call(
    ///         FunctionCall::name("ft_transfer_call")
    ///             .args_json(&json!({
    ///                 "receiver_id": "receiver.near",
    ///                 "amount": "1000",
    ///                 "msg": "...",
    ///             }))
    ///             .attach_deposit(NearToken::from_yoctonear(1))
    ///             .gas(Gas::from_tgas(100)),
    ///     );
    /// ```
    #[inline]
    pub fn function_call(self, action: impl Into<FunctionCall>) -> Self {
        self.add_action(action.into())
    }

    /// Add given action to this promise.
    #[inline]
    pub fn add_action(mut self, action: impl Into<NearAction>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Returns whether the promise is no-op, i.e. list of actions is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_near_promise::{AccountIdRef, NearPromise};
    /// # let account_id = AccountIdRef::new_or_panic("account.near");
    /// let p = NearPromise::new(account_id);
    ///
    /// assert!(p.is_empty());
    /// ```
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Returns total NEAR deposit for all actions in this promise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_near_promise::{
    /// #    AccountIdRef, NearPromise, NearToken,
    /// #    actions::FunctionCall,
    /// # };
    /// # let contract_id = AccountIdRef::new_or_panic("ft.near");
    /// let p = NearPromise::new(contract_id)
    ///     .transfer(NearToken::from_near(1))
    ///     .function_call(
    ///         FunctionCall::name("foo")
    ///             .attach_deposit(NearToken::from_near(2)),
    ///     );
    ///
    /// assert_eq!(p.total_deposit(), NearToken::from_near(3));
    /// ```
    #[inline]
    pub fn total_deposit(&self) -> NearToken {
        self.actions
            .iter()
            .map(NearAction::deposit)
            .fold(NearToken::ZERO, NearToken::saturating_add)
    }

    /// Returns an esitmate of mininum gas required to execute all
    /// actions in this promise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_near_promise::{
    /// #    AccountIdRef, Gas, NearPromise,
    /// #    actions::FunctionCall,
    /// # };
    /// # let contract_id = AccountIdRef::new_or_panic("ft.near");
    /// let p = NearPromise::new(contract_id)
    ///     .function_call(FunctionCall::name("foo").gas(Gas::from_tgas(20)))
    ///     .function_call(FunctionCall::name("bar").gas(Gas::from_tgas(35)));
    ///
    /// assert_eq!(p.estimate_gas(), Gas::from_tgas(55));
    /// ```
    #[inline]
    pub fn estimate_gas(&self) -> Gas {
        self.actions
            .iter()
            .map(NearAction::estimate_gas)
            .fold(Gas::from_gas(0), Gas::saturating_add)
    }

    #[cfg(feature = "near-contract")]
    /// Build [`Promise`] for execution
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_near_promise::{AccountIdRef, NearPromise, NearToken};
    /// # let account_id = AccountIdRef::new_or_panic("ft.near");
    /// let p = NearPromise::new(account_id)
    ///     .transfer(NearToken::from_near(1));
    ///
    /// // build and detach
    /// p.build().detach();
    /// ```
    pub fn build(self) -> ::near_sdk::Promise {
        let mut p = ::near_sdk::Promise::new(self.receiver_id);

        if let Some(refund_to) = self.refund_to {
            p = p.refund_to(refund_to);
        }

        self.actions
            .into_iter()
            .fold(p, |p, action| action.append(p))
    }
}
