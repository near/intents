use defuse_admin_utils::prefixed::{MessagePrefix, PrefixedMessage};
use defuse_near_utils::{CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID};
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_contract_standards::fungible_token::metadata::{
    FT_METADATA_SPEC, FungibleTokenMetadata, FungibleTokenMetadataProvider,
};
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_contract_standards::fungible_token::{
    FungibleToken, FungibleTokenCore, FungibleTokenResolver,
};
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::borsh::BorshSerialize;
use near_sdk::collections::LazyOption;
use near_sdk::json_types::U128;
use near_sdk::{
    AccountId, AccountIdRef, BorshStorageKey, Gas, NearToken, PanicOnDefault, PromiseOrValue, env,
    ext_contract, log, near, require,
};

pub const FT_TRANSFER_GAS: Gas = Gas::from_tgas(10);
pub const FT_REFUND_GAS: Gas = Gas::from_tgas(10);

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
    wrapped_token_id: AccountId,
}

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    FungibleToken,
    Metadata,
}

#[near]
impl Contract {
    #[init]
    #[must_use]
    pub fn new_default_meta(
        owner_id: AccountId,
        symbol: &str,
        wrapped_token_id: AccountId,
    ) -> Self {
        Self::new(
            owner_id,
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: format!("Wrapped token of: {wrapped_token_id}"),
                symbol: symbol.to_string(),
                icon: None,
                reference: None,
                reference_hash: None,
                decimals: 24,
            },
            wrapped_token_id,
        )
    }

    #[init]
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn new(
        owner_id: AccountId,
        metadata: FungibleTokenMetadata,
        wrapped_token_id: AccountId,
    ) -> Self {
        require!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        let mut this = Self {
            token: FungibleToken::new(StorageKey::FungibleToken),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            wrapped_token_id,
        };
        this.token.internal_register_account(&owner_id);
        this.token.internal_deposit(&owner_id, 0);

        near_contract_standards::fungible_token::events::FtMint {
            owner_id: &owner_id,
            amount: 0.into(),
            memo: Some("new tokens are minted"),
        }
        .emit();

        this
    }

    #[private]
    pub fn ft_resolve_unwrap(&mut self, account_id: &AccountId, amount: u128) {
        let unused_amount = match env::promise_result(0) {
            near_sdk::PromiseResult::Successful(value) => {
                if let Ok(unused_amount) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    std::cmp::min(amount, unused_amount.0)
                } else {
                    amount
                }
            }
            near_sdk::PromiseResult::Failed => amount,
        };

        self.token.internal_deposit(account_id, unused_amount);
    }
}

#[near]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        self.token.ft_transfer(receiver_id, amount, memo);
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let parsed = PrefixedMessage::<UnwrapPrefix, AccountId>::from(msg.as_str());
        match parsed {
            PrefixedMessage::NoMatch(m) => {
                self.token
                    .ft_transfer_call(receiver_id, amount, memo, m.to_string())
            }
            PrefixedMessage::Matched {
                suffix: receiver_id_from_msg,
                rest,
                _marker,
            } => {
                if receiver_id != *CURRENT_ACCOUNT_ID {
                    env::panic_str(
                        "Given msg indicated the will to unwrap the token, but the receiver address is not the contract address",
                    )
                }
                let previous_owner = &*PREDECESSOR_ACCOUNT_ID;

                self.token
                    .internal_withdraw(&receiver_id_from_msg, amount.0);

                ext_ft_core::ext(self.wrapped_token_id.clone())
                    .ft_transfer_call(receiver_id_from_msg, amount, memo, rest.to_string())
                    .then(
                        Contract::ext(CURRENT_ACCOUNT_ID.clone())
                            .with_static_gas(FT_REFUND_GAS)
                            .ft_resolve_unwrap(previous_owner, amount.0), // FIXME: Does this require ownership management because it's async, so we can't pass a reference?
                    )
                    .into()
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
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        #[allow(unused_variables)] sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let parsed = PrefixedMessage::<TransferPrefix, AccountId>::from(msg.as_str());
        let unused_amount = match parsed {
            PrefixedMessage::NoMatch(_) => amount,
            PrefixedMessage::Matched {
                suffix: receiver_id,
                rest: _,
                _marker,
            } => {
                self.token.internal_deposit(&receiver_id, amount.0);
                0.into()
            }
        };
        PromiseOrValue::Value(unused_amount)
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
        let (used_amount, burned_amount) =
            self.token
                .internal_ft_resolve_transfer(&sender_id, receiver_id, amount);
        if burned_amount > 0 {
            log!("Account @{} burned {}", sender_id, burned_amount);
        }
        used_amount.into()
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
        #[allow(unused_variables)]
        if let Some((account_id, balance)) = self.token.internal_storage_unregister(force) {
            log!("Closed @{} with {}", account_id, balance);
            true
        } else {
            false
        }
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
        self.metadata.get().unwrap()
    }
}

#[ext_contract(ext_wrap_token)]
pub trait WrappedToken: FungibleTokenCore + FungibleTokenResolver + StorageManagement {
    fn wrapped_token(&self) -> &AccountIdRef;
}

struct UnwrapPrefix;

impl MessagePrefix for UnwrapPrefix {
    const PREFIX: &'static str = "UNWRAP_TO";
}

struct TransferPrefix;

impl MessagePrefix for TransferPrefix {
    const PREFIX: &'static str = "TRANSFER_TO";
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
