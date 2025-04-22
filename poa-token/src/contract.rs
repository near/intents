use defuse_admin_utils::full_access_keys::FullAccessKeys;
use defuse_near_utils::{CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID};
use near_contract_standards::{
    fungible_token::{
        FungibleToken, FungibleTokenCore, FungibleTokenResolver,
        events::{FtBurn, FtMint},
        metadata::{FT_METADATA_SPEC, FungibleTokenMetadata, FungibleTokenMetadataProvider},
    },
    storage_management::{StorageBalance, StorageBalanceBounds, StorageManagement},
};
use near_plugins::{Ownable, events::AsEvent, only, ownable::OwnershipTransferred};
use near_sdk::{
    AccountId, BorshStorageKey, NearToken, PanicOnDefault, Promise, PromiseOrValue, PublicKey,
    assert_one_yocto, borsh::BorshSerialize, env, json_types::U128, near, require, store::Lazy,
};

use crate::{PoaFungibleToken, WITHDRAW_MEMO_PREFIX};

#[near(contract_state)]
#[derive(Ownable, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: Lazy<FungibleTokenMetadata>,
}

#[near]
impl Contract {
    #[init]
    #[allow(dead_code)]
    pub fn new(owner_id: Option<AccountId>, metadata: Option<FungibleTokenMetadata>) -> Self {
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
            self.ft_withdraw(&PREDECESSOR_ACCOUNT_ID, amount, memo);
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
        self.token.ft_transfer_call(receiver_id, amount, memo, msg)
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
    #[cfg_attr(feature = "no-registration", only(self, owner))]
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
    #[payable]
    fn add_full_access_key(&mut self, public_key: PublicKey) -> Promise {
        assert_one_yocto();
        Promise::new(CURRENT_ACCOUNT_ID.clone()).add_full_access_key(public_key)
    }

    #[only(self, owner)]
    #[payable]
    fn delete_key(&mut self, public_key: PublicKey) -> Promise {
        assert_one_yocto();
        Promise::new(CURRENT_ACCOUNT_ID.clone()).delete_key(public_key)
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "::near_sdk::borsh")]
enum Prefix {
    FungibleToken,
    Metadata,
}
