use defuse_admin_utils::full_access_keys::FullAccessKeys;
use defuse_near_utils::{CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID};
use near_contract_standards::{
    fungible_token::{
        FungibleToken, FungibleTokenCore, FungibleTokenResolver,
        core::ext_ft_core,
        events::{FtBurn, FtMint},
        metadata::{FT_METADATA_SPEC, FungibleTokenMetadata, FungibleTokenMetadataProvider},
    },
    storage_management::{StorageBalance, StorageBalanceBounds, StorageManagement},
};
use near_plugins::{Ownable, events::AsEvent, only, ownable::OwnershipTransferred};
use near_sdk::{
    AccountId, AccountIdRef, BorshStorageKey, Gas, NearToken, PanicOnDefault, Promise,
    PromiseOrValue, PublicKey, assert_one_yocto,
    borsh::{BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    near, require,
    store::Lazy,
};

use crate::{CanWrapToken, PoaFungibleToken, WITHDRAW_MEMO_PREFIX, prefixed::PrefixedMessage};

const FT_UNWRAP_GAS: Gas = Gas::from_tgas(10);
const DO_WRAP_TOKEN_GAS: Gas = Gas::from_tgas(10);

#[derive(BorshSerialize, BorshDeserialize)]
#[borsh(crate = "::near_sdk::borsh")]
pub struct ContractWithoutOmniWrapping {
    token: FungibleToken,
    metadata: Lazy<FungibleTokenMetadata>,
}

#[near(contract_state)]
#[derive(Ownable, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: Lazy<FungibleTokenMetadata>,
    wrapped_token: Option<AccountId>,
}

#[near]
impl Contract {
    #[init]
    pub fn new(
        owner_id: Option<AccountId>,
        metadata: Option<FungibleTokenMetadata>,
        wrapped_token: Option<AccountId>,
    ) -> Self {
        let metadata = metadata.unwrap_or_else(|| FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: Default::default(),
            symbol: Default::default(),
            icon: Default::default(),
            reference: Default::default(),
            reference_hash: Default::default(),
            decimals: Default::default(),
        });
        metadata.assert_valid();

        let contract = Self {
            token: FungibleToken::new(Prefix::FungibleToken),
            metadata: Lazy::new(Prefix::Metadata, metadata),
            wrapped_token,
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

    #[private]
    #[init(ignore_state)]
    pub fn upgrade_wrap_for_omni_bridge() -> Self {
        let old_state: ContractWithoutOmniWrapping =
            env::state_read().expect("Deserializing old state failed");

        Self {
            token: old_state.token,
            metadata: old_state.metadata,
            wrapped_token: None,
        }
    }

    #[only(self, owner)]
    pub fn clear_wrapped_token_account_id(&mut self) {
        self.wrapped_token = None;
    }

    #[only(self, owner)]
    pub fn set_wrapped_token_account_id(&mut self, token_account_id: AccountId) -> Promise {
        if self.wrapped_token.is_some() {
            env::panic_str("Wrapped token is already set")
        }

        ext_ft_core::ext(token_account_id.clone())
            .ft_balance_of(CURRENT_ACCOUNT_ID.clone())
            .then(
                Self::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(DO_WRAP_TOKEN_GAS)
                    .do_set_wrapped_token_account_id(token_account_id),
            )
    }

    #[only(self, owner)]
    pub fn do_set_wrapped_token_account_id(&mut self, token_account_id: AccountId) {
        let parsed_balance = match env::promise_result(0) {
            near_sdk::PromiseResult::Successful(balance_bytes) => {
                if let Ok(balance) = near_sdk::serde_json::from_slice::<U128>(&balance_bytes) {
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
        require!(parsed_balance >= self_total_supply);

        self.wrapped_token = Some(token_account_id);
    }

    #[only(self, owner)]
    pub fn ft_resolve_unwrap(&mut self, account_id: &AccountId, amount: u128, is_call: bool) {
        let unused_amount = match env::promise_result(0) {
            near_sdk::PromiseResult::Successful(value) => {
                if let Ok(unused_amount) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    std::cmp::min(amount, unused_amount.0)
                } else {
                    amount
                }
            }
            near_sdk::PromiseResult::Failed => {
                // TODO: understand this logic
                if is_call {
                    // do not refund on failed `ft_transfer_call` due to
                    // NEP-141 vulnerability: `ft_resolve_transfer` fails to
                    // read result of `ft_on_transfer` due to insufficient gas
                    amount
                } else {
                    0
                }
            }
        };

        self.token.internal_deposit(account_id, unused_amount);
    }
}

impl Contract {
    fn do_unwrap_and_transfer(
        &mut self,
        token_destination: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
        wrapped_token_id: AccountId,
    ) -> PromiseOrValue<U128> {
        let previous_owner = &*PREDECESSOR_ACCOUNT_ID;

        self.ft_withdraw(&PREDECESSOR_ACCOUNT_ID, amount, None);

        let is_call = !msg.is_empty();

        ext_ft_core::ext(wrapped_token_id.to_owned())
            .ft_transfer_call(token_destination, amount, memo, msg)
            .then(
                Contract::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(FT_UNWRAP_GAS)
                    .ft_resolve_unwrap(previous_owner, amount.0, is_call),
            )
            .into()
    }
}

#[near]
impl PoaFungibleToken for Contract {
    #[only(self, owner)]
    #[payable]
    fn set_metadata(&mut self, metadata: FungibleTokenMetadata) {
        assert_one_yocto();
        metadata.assert_valid();
        self.metadata.set(metadata);
    }

    #[only(self, owner)]
    #[payable]
    fn ft_deposit(&mut self, owner_id: AccountId, amount: U128, memo: Option<String>) {
        if self.wrapped_token.is_some() {
            env::panic_str("This PoA token was migrated to OmniBridge. No deposits are possible.");
        }

        self.token.storage_deposit(Some(owner_id.clone()), None);
        self.token.internal_deposit(&owner_id, amount.into());
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
    fn wrapped_token(&self) -> Option<&AccountIdRef> {
        self.wrapped_token.as_deref()
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
                .map_or(false, |memo| memo.starts_with(WITHDRAW_MEMO_PREFIX))
        {
            match self.wrapped_token() {
                Some(_) => env::panic_str("This PoA token was migrated to OmniBridge"),
                None => self.ft_withdraw(&PREDECESSOR_ACCOUNT_ID, amount, memo),
            }
        } else {
            self.token.ft_transfer(receiver_id, amount, memo)
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
        let Some(wrapped_token_id) = self.wrapped_token().map(|id| id.to_owned()) else {
            return self.token.ft_transfer_call(receiver_id, amount, memo, msg);
        };

        let parsed = PrefixedMessage::from(msg.as_str());
        match parsed {
            PrefixedMessage::NoMatch(_) => {
                self.do_unwrap_and_transfer(receiver_id, amount, memo, msg, wrapped_token_id)
            }
            PrefixedMessage::Matched {
                account_id: receiver_from_msg,
                rest,
            } => self.do_unwrap_and_transfer(
                receiver_from_msg,
                amount,
                memo,
                rest.to_string(),
                wrapped_token_id,
            ),
            PrefixedMessage::MatchedNoMessage(account_id) => {
                self.ft_transfer(account_id, amount, memo);
                PromiseOrValue::Value(0.into())
            }
        }
    }

    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.token.ft_balance_of(account_id)
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
        self.token
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
        let wrapped_token_id = match &self.wrapped_token {
            Some(w) => w,
            None => env::panic_str("This function is supposed to be used only with a token"),
        };

        if wrapped_token_id != &*PREDECESSOR_ACCOUNT_ID {
            env::panic_str("Only the wrapping token contract can call this function")
        }

        let recipient = if msg.is_empty() {
            sender_id
        } else {
            use defuse_near_utils::UnwrapOrPanicError;
            msg.parse().unwrap_or_panic_display()
        };

        self.token.internal_deposit(&recipient, amount.0);

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
        self.token.storage_deposit(account_id, registration_only)
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        self.token.storage_withdraw(amount)
    }

    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        self.token.storage_unregister(force)
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        self.token.storage_balance_bounds()
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.token.storage_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.clone()
    }
}

impl Contract {
    fn ft_withdraw(&mut self, account_id: &AccountId, amount: U128, memo: Option<String>) {
        assert_one_yocto();
        require!(amount.0 > 0, "zero amount");
        self.token.internal_withdraw(account_id, amount.into());
        FtBurn {
            owner_id: account_id,
            amount,
            memo: memo.as_deref(),
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

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "::near_sdk::borsh")]
enum Prefix {
    FungibleToken,
    Metadata,
}
