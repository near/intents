use crate::{CanWrapToken, PoaFungibleToken, UNWRAP_PREFIX, WITHDRAW_MEMO_PREFIX};
use defuse_admin_utils::full_access_keys::FullAccessKeys;
use defuse_controller::ControllerUpgradable;
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, Lock, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicCtx,
    UnwrapOrPanicError, method_name,
};
use near_contract_standards::{
    fungible_token::{
        FungibleToken, FungibleTokenCore, FungibleTokenResolver,
        core::ext_ft_core,
        events::{FtBurn, FtMint},
        metadata::{
            FT_METADATA_SPEC, FungibleTokenMetadata, FungibleTokenMetadataProvider, ext_ft_metadata,
        },
    },
    storage_management::{StorageBalance, StorageBalanceBounds, StorageManagement},
};
use near_plugins::Ownable;
use near_plugins::{events::AsEvent, only, ownable::OwnershipTransferred};
use near_sdk::{
    AccountId, BorshStorageKey, Gas, NearToken, PanicOnDefault, Promise, PromiseOrValue, PublicKey,
    assert_one_yocto,
    borsh::{BorshDeserialize, BorshSerialize},
    env::{self},
    json_types::U128,
    near, require, serde_json,
    store::Lazy,
};

const STATE_MIGRATE_FUNCTION: &str = method_name!(Contract::state_migrate);
const STATE_MIGRATE_DEFAULT_GAS: Gas = Gas::from_tgas(5);

const FT_RESOLVE_UNWRAP_GAS: Gas = Gas::from_tgas(10);
const DO_WRAP_TOKEN_GAS: Gas = Gas::from_tgas(10);
const BALANCE_OF_GAS: Gas = Gas::from_tgas(10);
const METADATA_GET_TOKEN_GAS: Gas = Gas::from_tgas(40);
const METADATA_SET_TOKEN_GAS: Gas = Gas::from_tgas(50);

#[derive(BorshSerialize, BorshDeserialize)]
#[borsh(crate = "::near_sdk::borsh")]
pub struct LegacyPoATokenContract {
    token: FungibleToken,
    metadata: Lazy<FungibleTokenMetadata>,
}

#[near]
pub struct WrappableTokenState {
    token: FungibleToken,
    metadata: Lazy<FungibleTokenMetadata>,
    wrapped_token: Lazy<Option<AccountId>>,
}

#[near]
enum ContractState {
    WrappableToken(Lock<WrappableTokenState>),
}

#[near(contract_state)]
#[derive(Ownable, PanicOnDefault)]
pub struct Contract {
    inner: ContractState,
}

impl Contract {
    fn is_locked(&self) -> bool {
        match &self.inner {
            ContractState::WrappableToken(lock) => lock.is_locked(),
        }
    }

    fn token(&self) -> &FungibleToken {
        match &self.inner {
            ContractState::WrappableToken(lock) => &lock.ignore_lock().token,
        }
    }

    fn token_mut(&mut self) -> &mut FungibleToken {
        match &mut self.inner {
            ContractState::WrappableToken(lock) => {
                &mut lock
                    .as_unlocked_mut()
                    .unwrap_or_panic_ctx("Locked for token mut")
                    .token
            }
        }
    }

    fn metadata(&self) -> &Lazy<FungibleTokenMetadata> {
        match &self.inner {
            ContractState::WrappableToken(lock) => &lock.ignore_lock().metadata,
        }
    }

    fn metadata_mut(&mut self) -> &mut Lazy<FungibleTokenMetadata> {
        match &mut self.inner {
            ContractState::WrappableToken(lock) => {
                &mut lock
                    .as_unlocked_mut()
                    .unwrap_or_panic_ctx("Locked for token ref")
                    .metadata
            }
        }
    }

    fn wrapped_token(&self) -> Option<&AccountId> {
        match &self.inner {
            ContractState::WrappableToken(lock) => (*lock.ignore_lock().wrapped_token).as_ref(),
        }
    }
}

#[near]
impl Contract {
    #[must_use]
    #[init]
    pub fn new(owner_id: Option<AccountId>, metadata: Option<FungibleTokenMetadata>) -> Self {
        let metadata = metadata.unwrap_or_else(|| FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: String::default(),
            symbol: String::default(),
            icon: Option::default(),
            reference: Option::default(),
            reference_hash: Option::default(),
            decimals: Default::default(),
        });
        metadata.assert_valid();

        let contract = Self {
            inner: ContractState::WrappableToken(Lock::new(
                WrappableTokenState {
                    token: FungibleToken::new(Prefix::FungibleToken),
                    metadata: Lazy::new(Prefix::Metadata, metadata),
                    wrapped_token: Lazy::new(Prefix::WrappedToken, None),
                },
                false,
            )),
        };

        let owner = owner_id.unwrap_or_else(|| PREDECESSOR_ACCOUNT_ID.clone());
        // Ownable::owner_set requires it to be a promise
        require!(!env::storage_write(
            contract.owner_storage_key(),
            owner.as_bytes()
        ));
        OwnershipTransferred {
            previous_owner: None,
            new_owner: Some(owner),
        }
        .emit();

        contract
    }

    #[must_use]
    #[private]
    #[init(ignore_state)]
    pub fn upgrade_to_versioned() -> Self {
        let old_state: LegacyPoATokenContract =
            env::state_read().expect("Deserializing old state failed");

        Self {
            inner: ContractState::WrappableToken(Lock::new(
                WrappableTokenState {
                    token: old_state.token,
                    metadata: old_state.metadata,
                    wrapped_token: Lazy::new(Prefix::WrappedToken, None),
                },
                false,
            )),
        }
    }

    // Note that we make this permission'ed because it can be disastrous if an attacker can change the number of decimals of an external contract,
    // and then sync the metadata (update our decimals), which will break all off-chain (and possibly on-chain) applications.
    #[only(self, owner)]
    #[payable]
    pub fn force_sync_wrapped_token_metadata(&mut self) -> Promise {
        let caller_id = env::predecessor_account_id();

        require!(
            env::attached_deposit() >= NearToken::from_yoctonear(1),
            "Requires attached deposit of exactly 1 yoctoNEAR or more"
        );

        let Some(wrapped_token) = self.wrapped_token().cloned() else {
            env::panic_str("This function is restricted to wrapped tokens")
        };

        ext_ft_metadata::ext(wrapped_token)
            .with_static_gas(METADATA_GET_TOKEN_GAS)
            .ft_metadata()
            .then(
                Self::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(METADATA_SET_TOKEN_GAS)
                    .with_attached_deposit(env::attached_deposit())
                    .do_sync_wrapped_metadata(caller_id),
            )
    }

    #[private]
    #[payable]
    pub fn do_sync_wrapped_metadata(&mut self, caller_id: AccountId) -> PromiseOrValue<()> {
        let near_sdk::PromiseResult::Successful(metadata_bytes) = env::promise_result(0) else {
            env::panic_str(
                "Setting metadata failed due to the promise failing at ft_metadata() call.",
            )
        };

        let incoming_metadata = serde_json::from_slice::<FungibleTokenMetadata>(&metadata_bytes)
            .map_err(|e| {
                format!("JSON: failed to parse Promise output as FungibleTokenMetadata: {e}")
            })
            .unwrap_or_panic();

        let initial_storage_usage = env::storage_usage();

        let to_set_metadata = FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: format!("Wrapped {}", incoming_metadata.name),
            symbol: format!("w{}", incoming_metadata.symbol),
            ..incoming_metadata
        };

        to_set_metadata.assert_valid();

        match &mut self.inner {
            ContractState::WrappableToken(lock) => {
                let metadata = &mut lock
                    .as_unlocked_mut()
                    .unwrap_or_panic_ctx("Smart contract seems to be locked")
                    .metadata;
                metadata.set(to_set_metadata);
                metadata.flush();
            }
        }

        let end_storage_usage = env::storage_usage();

        // Note that we use saturating sub here to prevent abuse. We do not refund Near for freed storage
        let storage_increase_byte_count = end_storage_usage.saturating_sub(initial_storage_usage);

        let storage_increase_cost = env::storage_byte_cost()
            .checked_mul(u128::from(storage_increase_byte_count))
            .ok_or("Storage cost calculation overflow")
            .unwrap_or_panic();

        let refund = env::attached_deposit()
            .checked_sub(storage_increase_cost)
            .ok_or_else(|| {
                format!(
                    "Insufficient attached deposit {}yN, required {}yN",
                    env::attached_deposit().as_yoctonear(),
                    storage_increase_cost.as_yoctonear(),
                )
            })
            .unwrap_or_panic();

        if refund > NearToken::from_yoctonear(0) {
            Promise::new(caller_id).transfer(refund).into()
        } else {
            PromiseOrValue::Value(())
        }
    }

    #[only(self, owner)]
    #[payable]
    pub fn set_wrapped_token_account_id(&mut self, token_account_id: AccountId) -> Promise {
        if !self.is_locked() {
            env::panic_str(
                "The contract must be locked first before setting the wrapped token target",
            )
        }

        let caller_id = env::predecessor_account_id();

        require!(
            self.wrapped_token().is_none(),
            "Wrapped token is already set"
        );

        ext_ft_core::ext(token_account_id.clone())
            .with_static_gas(BALANCE_OF_GAS)
            .ft_balance_of(CURRENT_ACCOUNT_ID.clone())
            .then(
                Self::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_attached_deposit(env::attached_deposit())
                    .with_static_gas(DO_WRAP_TOKEN_GAS)
                    .do_set_wrapped_token_account_id(token_account_id, caller_id),
            )
    }

    #[private]
    #[payable]
    pub fn do_set_wrapped_token_account_id(
        &mut self,
        token_account_id: AccountId,
        caller_id: AccountId,
    ) -> PromiseOrValue<()> {
        require!(
            self.wrapped_token().is_none(),
            "Wrapped token is already set"
        );

        let parsed_balance = match env::promise_result(0) {
            near_sdk::PromiseResult::Successful(balance_bytes) => {
                if let Ok(balance) = serde_json::from_slice::<U128>(&balance_bytes) {
                    balance
                } else {
                    env::panic_str(&format!(
                        "Setting token id {token_account_id} for contract {} failed due to bad balance bytes",
                        &*CURRENT_ACCOUNT_ID
                    ))
                }
            }
            near_sdk::PromiseResult::Failed => env::panic_str(
                "Setting token id {token_account_id} for contract {} failed due to failed promise",
            ),
        };

        let self_total_supply = self.ft_total_supply();
        require!(
            parsed_balance >= self_total_supply,
            format!(
                "Migration required that the wrapped token have sufficient balance to cover for the balance in this contract: {} < {}",
                parsed_balance.0, self_total_supply.0
            )
        );

        let initial_storage_usage = env::storage_usage();

        match &mut self.inner {
            ContractState::WrappableToken(lock) => {
                let wrapped_token = &mut lock
                    .as_locked_mut()
                    .unwrap_or_panic_ctx("Invariant broken. The contract is expected to be locked when updating the wrapped token id")
                    .wrapped_token;

                wrapped_token.set(Some(token_account_id));
                wrapped_token.flush();
            }
        }

        let end_storage_usage = env::storage_usage();

        let storage_increase_byte_count = end_storage_usage.saturating_sub(initial_storage_usage);

        let storage_increase_cost = env::storage_byte_cost()
            .checked_mul(u128::from(storage_increase_byte_count))
            .ok_or("Storage cost calculation overflow")
            .unwrap_or_panic();

        let refund = env::attached_deposit()
            .checked_sub(storage_increase_cost)
            .ok_or_else(|| {
                format!(
                    "Insufficient attached deposit {}yN, required {}yN",
                    env::attached_deposit().as_yoctonear(),
                    storage_increase_cost.as_yoctonear(),
                )
            })
            .unwrap_or_panic();

        if refund > NearToken::from_yoctonear(0) {
            Promise::new(caller_id).transfer(refund).into()
        } else {
            PromiseOrValue::Value(())
        }
    }

    /// Returns the amount of tokens that were used/unwrapped after requesting an unwrap
    #[private]
    pub fn ft_resolve_unwrap(
        &mut self,
        sender_id: &AccountId,
        amount: U128,
        is_call: bool,
    ) -> U128 {
        let used = match env::promise_result(0) {
            near_sdk::PromiseResult::Successful(value) => {
                if is_call {
                    // `ft_transfer_call` returns successfully transferred amount
                    if let Ok(unused_amount) = near_sdk::serde_json::from_slice::<U128>(&value) {
                        std::cmp::min(amount, unused_amount).0
                    } else {
                        amount.0
                    }
                } else if value.is_empty() {
                    // `ft_transfer` returns empty result on success
                    amount.0
                } else {
                    0
                }
            }
            near_sdk::PromiseResult::Failed => {
                if is_call {
                    // do not refund on failed `ft_transfer_call` due to
                    // NEP-141 vulnerability: `ft_resolve_transfer` fails to
                    // read result of `ft_on_transfer` due to insufficient gas
                    amount.0
                } else {
                    0
                }
            }
        };

        let to_refund = amount.0.saturating_sub(used);

        if to_refund > 0 {
            self.token_mut().internal_deposit(sender_id, to_refund);
            FtMint {
                owner_id: sender_id,
                amount,
                memo: Some("refund for unwrap"),
            }
            .emit();
        }

        used.into()
    }

    pub fn lock_contract(&mut self) {
        match &mut self.inner {
            ContractState::WrappableToken(lock) => {
                lock.lock()
                    .unwrap_or_panic_ctx("Contract is already locked");
            }
        }
    }

    pub fn unlock_contract(&mut self) {
        match &mut self.inner {
            ContractState::WrappableToken(lock) => {
                lock.unlock()
                    .unwrap_or_panic_ctx("Contract is already unlocked");
            }
        }
    }

    pub fn is_contract_locked(&self) -> bool {
        self.is_locked()
    }
}

impl Contract {
    fn unwrap_and_transfer(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<U128> {
        let Some(wrapped_token_id) = self.wrapped_token().cloned() else {
            env::panic_str("Unwrapping is only for wrapped tokens");
        };

        let sender_id = &*PREDECESSOR_ACCOUNT_ID;

        self.ft_withdraw(sender_id, amount, None);

        if let Some(inner_msg) = msg {
            ext_ft_core::ext(wrapped_token_id.clone())
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .ft_transfer_call(receiver_id, amount, memo, inner_msg)
                .then(
                    Contract::ext(CURRENT_ACCOUNT_ID.clone())
                        .with_static_gas(FT_RESOLVE_UNWRAP_GAS)
                        .ft_resolve_unwrap(sender_id, amount, true),
                )
                .into()
        } else {
            ext_ft_core::ext(wrapped_token_id.clone())
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .ft_transfer(receiver_id, amount, memo)
                .then(
                    Self::ext(CURRENT_ACCOUNT_ID.clone())
                        .ft_resolve_unwrap(sender_id, amount, false),
                )
                .into()
        }
    }
}

#[near]
impl PoaFungibleToken for Contract {
    #[only(self, owner)]
    #[payable]
    fn set_metadata(&mut self, metadata: FungibleTokenMetadata) {
        assert_one_yocto();
        metadata.assert_valid();
        self.metadata_mut().set(metadata);
    }

    #[only(self, owner)]
    #[payable]
    fn ft_deposit(&mut self, owner_id: AccountId, amount: U128, memo: Option<String>) {
        require!(
            self.wrapped_token().is_none(),
            "This PoA token was migrated to OmniBridge. No deposits are possible.",
        );

        self.token_mut()
            .storage_deposit(Some(owner_id.clone()), None);

        self.token_mut().internal_deposit(&owner_id, amount.into());

        FtMint {
            owner_id: &owner_id,
            amount,
            memo: memo.as_deref(),
        }
        .emit();
    }
}

#[near]
impl CanWrapToken for Contract {
    fn wrapped_token(&self) -> Option<&AccountId> {
        self.wrapped_token()
    }
}

#[near]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        // A special case we created to handle withdrawals:
        // If the receiver id is the token contract id, we burn these tokens by calling ft_withdraw,
        // which will reduce the balance and emit an FtBurn event.
        if receiver_id == *CURRENT_ACCOUNT_ID
            && memo
                .as_deref()
                .is_some_and(|memo| memo.starts_with(WITHDRAW_MEMO_PREFIX))
        {
            require!(
                self.wrapped_token().is_none(),
                "This PoA token was migrated to OmniBridge"
            );

            self.ft_withdraw(&PREDECESSOR_ACCOUNT_ID, amount, memo.as_deref());
        } else {
            self.token_mut().ft_transfer(receiver_id, amount, memo);
        }
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert_one_yocto();

        if receiver_id != *CURRENT_ACCOUNT_ID {
            return self
                .token_mut()
                .ft_transfer_call(receiver_id, amount, memo, msg);
        }

        if self.wrapped_token().is_none() {
            return self
                .token_mut()
                .ft_transfer_call(receiver_id, amount, memo, msg);
        };

        let Some(rest) = msg.strip_prefix(UNWRAP_PREFIX) else {
            // In the case when our custom conditions were not met,
            // we should keep backwards compatibility with NEP-141 standard,
            // so that other protocols can interact with this token as a
            // regular Fungible Token.
            //
            // We could have let the remaining Promises (i.e. ft_on_transfer()
            // and ft_resolve_transfer()) go though, but we make a shortcut
            // and save gas, since we know what is going to happen anyway.
            //
            // This is the expected behavior from NEP-141 token standard
            // in both cases: `deposits` feature enabled and not
            return PromiseOrValue::Value(U128(0));
        };

        if let Some((receiver_id_from_msg, msg)) = rest.split_once(':') {
            let receiver_id_from_msg = receiver_id_from_msg
                .parse::<AccountId>()
                .map_err(|e| format!("Failed to parse account id `{receiver_id_from_msg}`: {e}"))
                .unwrap_or_panic_display();

            self.unwrap_and_transfer(receiver_id_from_msg, amount, memo, Some(msg.to_string()))
        } else {
            let receiver_id_from_msg: AccountId = rest
                .parse()
                .map_err(|e| format!("Failed to parse account id `{rest}``: {e}"))
                .unwrap_or_panic_display();
            self.unwrap_and_transfer(receiver_id_from_msg, amount, memo, None)
        }
    }

    fn ft_total_supply(&self) -> U128 {
        self.token().ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.token().ft_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenResolver for Contract {
    #[private]
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        self.token_mut()
            .ft_resolve_transfer(sender_id, receiver_id, amount)
    }
}

#[cfg(feature = "deposits")]
#[near]
impl near_contract_standards::fungible_token::receiver::FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        require!(
            self.wrapped_token().is_some(),
            "This function is supposed to be used only with a token"
        );

        require!(
            self.wrapped_token()
                .as_ref()
                .is_some_and(|t| &*PREDECESSOR_ACCOUNT_ID == *t),
            "Only the wrapped token can be the caller of this function",
        );

        let recipient = if msg.is_empty() {
            sender_id
        } else {
            use defuse_near_utils::UnwrapOrPanicError;
            msg.parse().unwrap_or_panic_display()
        };

        self.token_mut().internal_deposit(&recipient, amount.0);

        PromiseOrValue::Value(0.into())
    }
}

#[near]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.token_mut()
            .storage_deposit(account_id, registration_only)
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        self.token_mut().storage_withdraw(amount)
    }

    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        self.token_mut().storage_unregister(force)
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        self.token().storage_balance_bounds()
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.token().storage_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata().as_ref().clone()
    }
}

impl Contract {
    fn ft_withdraw(&mut self, account_id: &AccountId, amount: U128, memo: Option<&str>) {
        assert_one_yocto();
        require!(amount.0 > 0, "zero amount");
        self.token_mut()
            .internal_withdraw(account_id, amount.into());
        FtBurn {
            owner_id: account_id,
            amount,
            memo,
        }
        .emit();
    }
}

#[near]
impl FullAccessKeys for Contract {
    #[only(self, owner)]
    fn add_full_access_key(&mut self, public_key: PublicKey) -> Promise {
        Promise::new(CURRENT_ACCOUNT_ID.clone()).add_full_access_key(public_key)
    }

    #[only(self, owner)]
    fn delete_key(&mut self, public_key: PublicKey) -> Promise {
        Promise::new(CURRENT_ACCOUNT_ID.clone()).delete_key(public_key)
    }
}

#[near]
impl ControllerUpgradable for Contract {
    #[only(self, owner)]
    #[payable]
    fn upgrade(
        &mut self,
        #[serializer(borsh)] code: Vec<u8>,
        #[serializer(borsh)] state_migration_gas: Option<Gas>,
    ) -> Promise {
        assert_one_yocto();
        Promise::new(CURRENT_ACCOUNT_ID.clone())
            .deploy_contract(code)
            .function_call(
                STATE_MIGRATE_FUNCTION.into(),
                Vec::new(),
                NearToken::from_yoctonear(0),
                state_migration_gas.unwrap_or(STATE_MIGRATE_DEFAULT_GAS),
            )
    }

    #[private]
    fn state_migrate(&mut self) {}
}

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "::near_sdk::borsh")]
enum Prefix {
    FungibleToken,
    Metadata,
    WrappedToken,
}
