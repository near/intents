use std::{borrow::Cow, collections::BTreeMap};

use near_contract_standards::non_fungible_token;
use near_sdk::{
    AccountId, AccountIdRef, CryptoHash, Gas, NearToken, json_types::U128, near,
    state_init::StateInit,
};
use serde_with::{DisplayFromStr, serde_as};

use crate::{
    DefuseError, Result,
    accounts::AccountEvent,
    amounts::Amounts,
    engine::{Engine, Inspector, State},
    events::DefuseEvent,
    intents::{ExecutableIntent, IntentEvent},
    tokens::TransferEvent,
    tokens::{MT_ON_TRANSFER_GAS_DEFAULT, MT_ON_TRANSFER_GAS_MIN},
};

#[must_use]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct NotifyOnTransfer {
    /// Optionally initialize the receiver's contract (Deterministic AccountId)
    /// via [`state_init`](https://github.com/near/NEPs/blob/master/neps/nep-0616.md#stateinit-action)
    /// right before calling `mt_on_transfer()` (in the same receipt).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_init: Option<StateInit>,

    /// Message to pass to `mt_on_transfer`
    pub msg: String,

    /// Minimum gas for `mt_on_transfer()`
    ///
    /// Remaining gas will be distributed evenly across all Function Call
    /// Promises created during execution of current receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gas: Option<Gas>,
}

impl NotifyOnTransfer {
    pub const fn new(msg: String) -> Self {
        Self {
            state_init: None,
            msg,
            min_gas: None,
        }
    }

    pub fn with_state_init(mut self, state_init: StateInit) -> Self {
        self.state_init = Some(state_init);
        self
    }

    pub const fn with_min_gas(mut self, min_gas: Gas) -> Self {
        self.min_gas = Some(min_gas);
        self
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Transfer a set of tokens from the signer to a specified account id, within the intents contract.
pub struct Transfer {
    pub receiver_id: AccountId,

    #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
    pub tokens: Amounts,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Optionally notify receiver_id via `mt_on_transfer()`
    ///
    /// NOTE: `min_gas` is adjusted with following values:
    /// * minimum: 5TGas
    /// * default: 30TGas
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub notification: Option<NotifyOnTransfer>,
}

impl ExecutableIntent for Transfer {
    fn execute_intent<S, I>(
        self,
        sender_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        if sender_id == self.receiver_id || self.tokens.is_empty() {
            return Err(DefuseError::InvalidIntent);
        }

        engine
            .inspector
            .on_event(DefuseEvent::Transfer(Cow::Borrowed(
                [IntentEvent::new(
                    AccountEvent::new(sender_id, TransferEvent::from(&self)),
                    intent_hash,
                )]
                .as_slice(),
            )));

        engine
            .state
            .internal_sub_balance(sender_id, self.tokens.clone())?;
        engine
            .state
            .internal_add_balance(self.receiver_id.clone(), self.tokens.clone())?;

        if let Some(mut notification) = self.notification {
            notification.min_gas = Some(
                notification
                    .min_gas
                    .unwrap_or(MT_ON_TRANSFER_GAS_DEFAULT)
                    .max(MT_ON_TRANSFER_GAS_MIN),
            );

            engine
                .state
                .notify_on_transfer(sender_id, self.receiver_id, self.tokens, notification);
        }

        Ok(())
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Withdraw given FT tokens from the intents contract to a given external account id (external being outside of intents).
pub struct FtWithdraw {
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub amount: U128,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Message to pass to `ft_transfer_call`. Otherwise, `ft_transfer` will be used.
    /// NOTE: No refund will be made in case of insufficient `storage_deposit`
    /// on `token` for `receiver_id`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,

    /// Optionally make `storage_deposit` for `receiver_id` on `token`.
    /// The amount will be subtracted from user's NEP-141 `wNEAR` balance.
    /// NOTE: the `wNEAR` will not be refunded in case of fail
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_deposit: Option<NearToken>,

    /// Optional minimum required Near gas for created Promise to succeed:
    /// * `ft_transfer`:      minimum: 15TGas, default: 15TGas
    /// * `ft_transfer_call`: minimum: 30TGas, default: 50TGas
    ///
    /// Remaining gas will be distributed evenly across all Function Call
    /// Promises created during execution of current receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gas: Option<Gas>,
}

impl FtWithdraw {
    const FT_TRANSFER_GAS_MIN: Gas = Gas::from_tgas(15);
    const FT_TRANSFER_GAS_DEFAULT: Gas = Gas::from_tgas(15);

    /// Taken from [near-contract-standards](https://github.com/near/near-sdk-rs/blob/985c16b8fffc623096d0b7e60b26746842a2d712/near-contract-standards/src/fungible_token/core_impl.rs#L137)
    const FT_TRANSFER_CALL_GAS_MIN: Gas = Gas::from_tgas(30);
    const FT_TRANSFER_CALL_GAS_DEFAULT: Gas = Gas::from_tgas(50);

    /// Returns whether it's `ft_transfer_call()`
    #[inline]
    pub const fn is_call(&self) -> bool {
        self.msg.is_some()
    }

    /// Returns minimum required gas
    #[inline]
    pub fn min_gas(&self) -> Gas {
        let (min, default) = if self.is_call() {
            (
                Self::FT_TRANSFER_CALL_GAS_MIN,
                Self::FT_TRANSFER_CALL_GAS_DEFAULT,
            )
        } else {
            (Self::FT_TRANSFER_GAS_MIN, Self::FT_TRANSFER_GAS_DEFAULT)
        };

        self.min_gas
            .unwrap_or(default)
            // We need to set hard minimum for gas to prevent loss of funds
            // due to insufficient gas:
            // 1. We don't refund wNEAR taken for `storage_deposit()`,
            //    which is executed in the same receipt as `ft_transfer[_call]()`
            // 2. We don't refund if `ft_transfer_call()` Promise fails
            .max(min)
    }
}

impl ExecutableIntent for FtWithdraw {
    #[inline]
    fn execute_intent<S, I>(
        self,
        owner_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        engine
            .inspector
            .on_event(DefuseEvent::FtWithdraw(Cow::Borrowed(
                [IntentEvent::new(
                    AccountEvent::new(owner_id, Cow::Borrowed(&self)),
                    intent_hash,
                )]
                .as_slice(),
            )));

        engine.state.ft_withdraw(owner_id, self)
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Withdraw given NFT tokens from the intents contract to a given external account id (external being outside of intents).
pub struct NftWithdraw {
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub token_id: non_fungible_token::TokenId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Message to pass to `nft_transfer_call`. Otherwise, `nft_transfer` will be used.
    /// NOTE: No refund will be made in case of insufficient `storage_deposit`
    /// on `token` for `receiver_id`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,

    /// Optionally make `storage_deposit` for `receiver_id` on `token`.
    /// The amount will be subtracted from user's NEP-141 `wNEAR` balance.
    /// NOTE: the `wNEAR` will not be refunded in case of fail
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_deposit: Option<NearToken>,

    /// Optional minimum required Near gas for created Promise to succeed:
    /// * `nft_transfer`:      minimum: 15TGas, default: 15TGas
    /// * `nft_transfer_call`: minimum: 30TGas, default: 50TGas
    ///
    /// Remaining gas will be distributed evenly across all Function Call
    /// Promises created during execution of current receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gas: Option<Gas>,
}

impl NftWithdraw {
    const NFT_TRANSFER_GAS_MIN: Gas = Gas::from_tgas(15);
    const NFT_TRANSFER_GAS_DEFAULT: Gas = Gas::from_tgas(15);

    /// Taken from [near-contract-standards](https://github.com/near/near-sdk-rs/blob/985c16b8fffc623096d0b7e60b26746842a2d712/near-contract-standards/src/non_fungible_token/core/core_impl.rs#L396)
    const NFT_TRANSFER_CALL_GAS_MIN: Gas = Gas::from_tgas(30);
    const NFT_TRANSFER_CALL_GAS_DEFAULT: Gas = Gas::from_tgas(50);

    /// Returns whether it's `nft_transfer_call()`
    #[inline]
    pub const fn is_call(&self) -> bool {
        self.msg.is_some()
    }

    /// Returns minimum required gas
    #[inline]
    pub fn min_gas(&self) -> Gas {
        let (min, default) = if self.is_call() {
            (
                Self::NFT_TRANSFER_CALL_GAS_MIN,
                Self::NFT_TRANSFER_CALL_GAS_DEFAULT,
            )
        } else {
            (Self::NFT_TRANSFER_GAS_MIN, Self::NFT_TRANSFER_GAS_DEFAULT)
        };

        self.min_gas
            .unwrap_or(default)
            // We need to set hard minimum for gas to prevent loss of funds
            // due to insufficient gas:
            // 1. We don't refund wNEAR taken for `storage_deposit()`,
            //    which is executed in the same receipt as `nft_transfer[_call]()`
            // 2. We don't refund if `nft_transfer_call()` Promise fails
            .max(min)
    }
}

impl ExecutableIntent for NftWithdraw {
    #[inline]
    fn execute_intent<S, I>(
        self,
        owner_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        engine
            .inspector
            .on_event(DefuseEvent::NftWithdraw(Cow::Borrowed(
                [IntentEvent::new(
                    AccountEvent::new(owner_id, Cow::Borrowed(&self)),
                    intent_hash,
                )]
                .as_slice(),
            )));

        engine.state.nft_withdraw(owner_id, self)
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Withdraw given MT tokens (i.e. [NEP-245](https://github.com/near/NEPs/blob/master/neps/nep-0245.md)) from the intents contract
/// to a given to an external account id (external being outside of intents).
///
/// If `msg` is given, `mt_batch_transfer_call()` will be used to transfer to the `receiver_id`. Otherwise, `mt_batch_transfer()` will be used.
pub struct MtWithdraw {
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub token_ids: Vec<defuse_nep245::TokenId>,
    pub amounts: Vec<U128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Message to pass to `mt_batch_transfer_call`. Otherwise, `mt_batch_transfer` will be used.
    /// NOTE: No refund will be made in case of insufficient `storage_deposit`
    /// on `token` for `receiver_id`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,

    /// Optionally make `storage_deposit` for `receiver_id` on `token`.
    /// The amount will be subtracted from user's NEP-141 `wNEAR` balance.
    /// NOTE: the `wNEAR` will not be refunded in case of fail
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_deposit: Option<NearToken>,

    /// Optional minimum required Near gas for created Promise to succeed per token:
    /// * `mt_batch_transfer`:      minimum: 20TGas, default: 20TGas
    /// * `mt_batch_transfer_call`: minimum: 35TGas, default: 50TGas
    ///
    /// Remaining gas will be distributed evenly across all Function Call
    /// Promises created during execution of current receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gas: Option<Gas>,
}

impl MtWithdraw {
    const MT_BATCH_TRANSFER_GAS_MIN: Gas = Gas::from_tgas(20);
    const MT_BATCH_TRANSFER_GAS_DEFAULT: Gas = Gas::from_tgas(20);

    const MT_BATCH_TRANSFER_CALL_GAS_MIN: Gas = Gas::from_tgas(35);
    const MT_BATCH_TRANSFER_CALL_GAS_DEFAULT: Gas = Gas::from_tgas(50);

    /// Returns whether it's `mt_batch_transfer_call()`
    #[inline]
    pub const fn is_call(&self) -> bool {
        self.msg.is_some()
    }

    /// Returns minimum required gas
    #[inline]
    pub fn min_gas(&self) -> Gas {
        let (min, default) = if self.is_call() {
            (
                Self::MT_BATCH_TRANSFER_CALL_GAS_MIN,
                Self::MT_BATCH_TRANSFER_CALL_GAS_DEFAULT,
            )
        } else {
            (
                Self::MT_BATCH_TRANSFER_GAS_MIN,
                Self::MT_BATCH_TRANSFER_GAS_DEFAULT,
            )
        };

        self.min_gas
            .unwrap_or(default)
            // We need to set hard minimum for gas to prevent loss of funds
            // due to insufficient gas:
            // 1. We don't refund wNEAR taken for `storage_deposit()`,
            //    which is executed in the same receipt as `mt_batch_transfer[_call]()`
            // 2. We don't refund if `mt_batch_transfer_call()` Promise fails
            .max(min)
    }
}

impl ExecutableIntent for MtWithdraw {
    #[inline]
    fn execute_intent<S, I>(
        self,
        owner_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        engine
            .inspector
            .on_event(DefuseEvent::MtWithdraw(Cow::Borrowed(
                [IntentEvent::new(
                    AccountEvent::new(owner_id, Cow::Borrowed(&self)),
                    intent_hash,
                )]
                .as_slice(),
            )));

        engine.state.mt_withdraw(owner_id, self)
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Withdraw native tokens (NEAR) from the intents contract to a given external account id (external being outside of intents).
/// This will subtract from the account's wNEAR balance, and will be sent to the account specified as native NEAR.
/// NOTE: the `wNEAR` will not be refunded in case of fail (e.g. `receiver_id`
/// account does not exist).
pub struct NativeWithdraw {
    pub receiver_id: AccountId,
    pub amount: NearToken,
}

impl ExecutableIntent for NativeWithdraw {
    #[inline]
    fn execute_intent<S, I>(
        self,
        owner_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        engine
            .inspector
            .on_event(DefuseEvent::NativeWithdraw(Cow::Borrowed(
                [IntentEvent::new(
                    AccountEvent::new(owner_id, Cow::Borrowed(&self)),
                    intent_hash,
                )]
                .as_slice(),
            )));

        engine.state.native_withdraw(owner_id, self)
    }
}

/// Make [NEP-145](https://nomicon.io/Standards/StorageManagement#nep-145)
/// `storage_deposit` for an `account_id` on `contract_id`.
/// The `amount` will be subtracted from user's NEP-141 `wNEAR` balance.
/// NOTE: the `wNEAR` will not be refunded in any case.
///
/// WARNING: use this intent only if paying storage_deposit is not a prerequisite
/// for other intents to succeed. If some intent (e.g. ft_withdraw) requires storage_deposit,
/// then use storage_deposit field of corresponding intent instead of adding a separate
/// `StorageDeposit` intent. This is due to the fact that intents that fire `Promise`s
/// are not guaranteed to be executed sequentially, in the order of the provided intents in
/// `DefuseIntents`.
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct StorageDeposit {
    pub contract_id: AccountId,
    #[serde(
        // There was field collision for `account_id` in `AccountEvent`,
        // but we keep it for backwards-compatibility
        alias = "account_id",
    )]
    pub deposit_for_account_id: AccountId,
    pub amount: NearToken,
}

impl ExecutableIntent for StorageDeposit {
    #[inline]
    fn execute_intent<S, I>(
        self,
        owner_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        engine
            .inspector
            .on_event(DefuseEvent::StorageDeposit(Cow::Borrowed(
                [IntentEvent::new(
                    AccountEvent::new(owner_id, Cow::Borrowed(&self)),
                    intent_hash,
                )]
                .as_slice(),
            )));

        engine.state.storage_deposit(owner_id, self)
    }
}

#[cfg(feature = "imt")]
pub mod imt {
    use crate::{
        Result,
        events::MaybeIntentEvent,
        tokens::{
            MT_ON_TRANSFER_GAS_DEFAULT, MT_ON_TRANSFER_GAS_MIN,
            imt::{ImtMintEvent, ImtTokens},
        },
    };
    use near_sdk::{AccountId, AccountIdRef, CryptoHash, near};
    use serde_with::{DisplayFromStr, serde_as};

    use std::{borrow::Cow, collections::BTreeMap};

    use crate::{
        DefuseError,
        accounts::AccountEvent,
        amounts::Amounts,
        engine::{Engine, Inspector, State},
        events::DefuseEvent,
        intents::{ExecutableIntent, IntentEvent, tokens::NotifyOnTransfer},
    };

    #[near(serializers = [borsh, json])]
    #[derive(Debug, Clone)]
    /// Mint a set of tokens from the signer to a specified account id, within the intents contract.
    pub struct ImtMint {
        /// Receiver of the minted tokens
        pub receiver_id: AccountId,

        /// The token_ids will be wrapped to bind the token ID to the
        /// minter authority (i.e. signer of this intent).
        /// The final string representation of the token will be as follows:
        /// `imt:<minter_id>:<token_id>`
        #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
        pub tokens: ImtTokens,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,

        /// Optionally notify receiver_id via `mt_on_transfer()`
        ///
        /// NOTE: `min_gas` is adjusted with following values:
        /// * minimum: 5TGas
        /// * default: 30TGas
        #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
        pub notification: Option<NotifyOnTransfer>,
    }

    impl ExecutableIntent for ImtMint {
        #[inline]
        fn execute_intent<S, I>(
            self,
            signer_id: &AccountIdRef,
            engine: &mut Engine<S, I>,
            intent_hash: CryptoHash,
        ) -> Result<()>
        where
            S: State,
            I: Inspector,
        {
            if self.tokens.is_empty() {
                return Err(DefuseError::InvalidIntent);
            }

            engine
                .inspector
                .on_event(DefuseEvent::ImtMint(Cow::Borrowed(
                    [MaybeIntentEvent::intent(
                        AccountEvent::new(signer_id, ImtMintEvent::from(&self)),
                        intent_hash,
                    )]
                    .as_slice(),
                )));

            let tokens = self.tokens.into_generic_tokens(signer_id)?;
            engine
                .state
                .mint(self.receiver_id.clone(), tokens.clone(), self.memo)?;

            if let Some(mut notification) = self.notification {
                notification.min_gas = Some(
                    notification
                        .min_gas
                        .unwrap_or(MT_ON_TRANSFER_GAS_DEFAULT)
                        .max(MT_ON_TRANSFER_GAS_MIN),
                );

                engine
                    .state
                    .notify_on_transfer(signer_id, self.receiver_id, tokens, notification);
            }

            Ok(())
        }
    }

    #[near(serializers = [borsh, json])]
    #[derive(Debug, Clone)]
    /// Burn a set of imt tokens, within the intents contract.
    pub struct ImtBurn {
        // The minter authority of the imt tokens
        pub minter_id: AccountId,

        /// The token_ids will be wrapped to bind the token ID to the
        /// minter authority. The final string representation of the
        /// token will be as follows:
        /// `imt:<minter_id>:<token_id>`
        #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
        pub tokens: ImtTokens,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    impl ExecutableIntent for ImtBurn {
        #[inline]
        fn execute_intent<S, I>(
            self,
            signer_id: &AccountIdRef,
            engine: &mut Engine<S, I>,
            intent_hash: CryptoHash,
        ) -> Result<()>
        where
            S: State,
            I: Inspector,
        {
            if self.tokens.is_empty() {
                return Err(DefuseError::InvalidIntent);
            }

            engine
                .inspector
                .on_event(DefuseEvent::ImtBurn(Cow::Borrowed(
                    [IntentEvent::new(
                        AccountEvent::new(signer_id, Cow::Borrowed(&self)),
                        intent_hash,
                    )]
                    .as_slice(),
                )));

            let tokens = self.tokens.into_generic_tokens(&self.minter_id)?;

            engine.state.burn(signer_id, tokens, self.memo)
        }
    }
}
