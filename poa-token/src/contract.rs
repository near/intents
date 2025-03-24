use defuse_admin_utils::{
    full_access_keys::FullAccessKeys,
    prefixed::{MessagePrefix, PrefixedMessage},
};
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

use crate::{CanWrapToken, PoaFungibleToken, WITHDRAW_MEMO_PREFIX};

pub const FT_TRANSFER_GAS: Gas = Gas::from_tgas(10);
pub const FT_REFUND_GAS: Gas = Gas::from_tgas(10);

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
    pub fn set_wrapped_token_account_id(&mut self, token_account_id: AccountId) {
        self.wrapped_token = Some(token_account_id);
    }

    #[only(self, owner)]
    pub fn clear_wrapped_token_account_id(&mut self) {
        self.wrapped_token = None;
    }

    #[only(self, owner)]
    pub fn ft_resolve_unwrap(&mut self, account_id: AccountId, amount: u128) {
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

        self.token.internal_deposit(&account_id, unused_amount);
    }
}

impl Contract {
    fn internal_ft_transfer_wrapped(
        &mut self,
        amount: U128,
        memo: Option<String>,
        msg: String,
        token_destination: AccountId,
        wrapped_token_id: AccountId,
    ) -> PromiseOrValue<U128> {
        let previous_owner = PREDECESSOR_ACCOUNT_ID.clone();

        self.ft_withdraw(&PREDECESSOR_ACCOUNT_ID, amount, None);
        ext_ft_core::ext(wrapped_token_id.to_owned())
            .ft_transfer_call(token_destination, amount, memo, msg)
            // FIXME: Do we need to refund on transfer failure?
            .then(
                Contract::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(FT_REFUND_GAS)
                    .ft_resolve_unwrap(previous_owner, amount.0),
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
        let wrapped_token = self.wrapped_token().map(|id| id.to_owned());
        match wrapped_token {
            Some(wrapped_token_id) => {
                let parsed = PrefixedMessage::<UnwrapPrefix, AccountId>::from(msg.as_str());
                match parsed {
                    PrefixedMessage::NoMatch(_) => self.internal_ft_transfer_wrapped(
                        amount,
                        memo,
                        msg,
                        receiver_id,
                        wrapped_token_id,
                    ),
                    PrefixedMessage::Matched {
                        suffix: receiver_from_msg,
                        rest,
                        _marker,
                    } => self.internal_ft_transfer_wrapped(
                        amount,
                        memo,
                        rest.to_string(),
                        receiver_from_msg,
                        wrapped_token_id,
                    ),
                }
            }
            None => self.token.ft_transfer_call(receiver_id, amount, memo, msg),
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

struct UnwrapPrefix;

impl MessagePrefix for UnwrapPrefix {
    const PREFIX: &'static str = "UNWRAP_TO";
}
