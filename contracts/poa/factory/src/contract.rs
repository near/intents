use core::iter;
use near_sdk::{
    json_types::Base64VecU8,
    store::{LookupMap, LookupSet},
};
use std::collections::{HashMap, HashSet};

use defuse_admin_utils::full_access_keys::FullAccessKeys;
use defuse_near_utils::gas_left;
use defuse_poa_token::ext_poa_fungible_token;
use near_contract_standards::fungible_token::{core::ext_ft_core, metadata::FungibleTokenMetadata};
use near_plugins::{
    AccessControlRole, AccessControllable, Pausable, access_control, access_control_any, pause,
};
use near_sdk::{
    AccountId, BorshStorageKey, Gas, NearToken, PanicOnDefault, Promise, PublicKey,
    assert_one_yocto,
    borsh::{BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    near, require,
    serde_json::{self, json},
    store::IterableSet,
};
use serde::{Deserialize, Serialize};

use crate::{FactoryEvent, PoaFactory, Withdrawal};

const POA_TOKEN_WASM: &[u8] = include_bytes!(std::env!("POA_TOKEN_WASM"));

pub const POA_TOKEN_INIT_BALANCE: NearToken = NearToken::from_near(3);
const POA_TOKEN_NEW_GAS: Gas = Gas::from_tgas(10);
const POA_TOKEN_FT_DEPOSIT_GAS: Gas = Gas::from_tgas(10);
/// Copied from `near_contract_standards::fungible_token::core_impl::GAS_FOR_FT_TRANSFER_CALL`
const POA_TOKEN_FT_TRANSFER_CALL_MIN_GAS: Gas = Gas::from_tgas(30);

#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    AccessControlRole,
)]
pub enum Role {
    DAO,
    TokenDeployer,
    TokenDepositer,
    PauseManager,
    UnpauseManager,
    TokenWithdrawer,
    OmniDepositer,
}

#[near(contract_state, contract_metadata())]
#[derive(Pausable, PanicOnDefault)]
#[access_control(role_type(Role))]
#[pausable(
    pause_roles(Role::DAO, Role::PauseManager),
    unpause_roles(Role::DAO, Role::UnpauseManager)
)]
pub struct Contract {
    tokens: IterableSet<String>,
    bridge_token_storage_deposit_required: NearToken,
    deposits: LookupSet<String>,
    withdrawals: LookupMap<String, Withdrawal>,
    omni_tokens: IterableSet<String>,
}

#[near]
impl Contract {
    #[must_use]
    #[init]
    #[allow(clippy::use_self)] // Due to a bug in clippy, even though we return Self, it still complains - happens in shared security analysis
    pub fn new(
        super_admins: HashSet<AccountId>,
        admins: HashMap<Role, HashSet<AccountId>>,
        grantees: HashMap<Role, HashSet<AccountId>>,
    ) -> Self {
        let mut contract = Self {
            tokens: IterableSet::new(Prefix::Tokens),
            bridge_token_storage_deposit_required: env::storage_byte_cost().saturating_mul(
                near_contract_standards::fungible_token::FungibleToken::new(b"t")
                    .account_storage_usage
                    .into(),
            ),
            deposits: LookupSet::new(Prefix::Deposits),
            withdrawals: LookupMap::new(Prefix::Withdrawals),
            omni_tokens: IterableSet::new(Prefix::OmniTokens),
        };

        let mut acl = contract.acl_get_or_init();
        require!(
            super_admins
                .into_iter()
                .all(|super_admin| acl.add_super_admin_unchecked(&super_admin))
                && admins
                    .into_iter()
                    .flat_map(|(role, admins)| iter::repeat(role).zip(admins))
                    .all(|(role, admin)| acl.add_admin_unchecked(role, &admin))
                && grantees
                    .into_iter()
                    .flat_map(|(role, grantees)| iter::repeat(role).zip(grantees))
                    .all(|(role, grantee)| acl.grant_role_unchecked(role, &grantee)),
            "failed to set roles"
        );
        contract
    }

    #[init(ignore_state)]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn migrate() -> Self {
        let old: OldContract = env::state_read().expect("failed to read old state");
        Self {
            tokens: old.tokens,
            bridge_token_storage_deposit_required: old.bridge_token_storage_deposit_required,
            deposits: LookupSet::new(Prefix::Deposits),
            withdrawals: LookupMap::new(Prefix::Withdrawals),
            omni_tokens: IterableSet::new(Prefix::OmniTokens),
        }
    }
}

#[derive(BorshDeserialize)]
#[borsh(crate = "::near_sdk::borsh")]
struct OldContract {
    tokens: IterableSet<String>,
    bridge_token_storage_deposit_required: NearToken,
}

#[near]
impl PoaFactory for Contract {
    #[pause]
    #[access_control_any(roles(Role::DAO, Role::TokenDeployer))]
    #[payable]
    fn deploy_token(&mut self, token: String, metadata: Option<FungibleTokenMetadata>) -> Promise {
        if let Some(metadata) = metadata.as_ref() {
            metadata.assert_valid();
        }

        let initial_storage = env::storage_usage();
        require!(self.tokens.insert(token.clone()), "token exists");
        let current_storage = env::storage_usage();
        require!(
            env::attached_deposit()
                >= POA_TOKEN_INIT_BALANCE.saturating_add(
                    env::storage_byte_cost()
                        .saturating_mul(current_storage.saturating_sub(initial_storage).into())
                ),
            "not enough deposit attached to deploy PoA token"
        );

        Promise::new(Self::token_id(token))
            .create_account()
            .transfer(POA_TOKEN_INIT_BALANCE)
            .deploy_contract(POA_TOKEN_WASM.to_vec())
            .function_call(
                "new".to_string(),
                serde_json::to_vec(&json!({
                    "metadata": metadata,
                }))
                .unwrap_or_else(|e| panic!("{e}")),
                NearToken::from_yoctonear(0),
                POA_TOKEN_NEW_GAS,
            )
    }

    #[pause]
    #[access_control_any(roles(Role::DAO, Role::TokenDeployer))]
    #[payable]
    fn set_metadata(&mut self, token: String, metadata: FungibleTokenMetadata) -> Promise {
        assert_one_yocto();
        require!(self.tokens.contains(&token), "token does not exist");

        ext_poa_fungible_token::ext(Self::token_id(token))
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .set_metadata(metadata)
    }

    #[pause]
    #[access_control_any(roles(Role::DAO, Role::TokenDepositer, Role::OmniDepositer))]
    #[payable]
    fn ft_deposit(
        &mut self,
        deposit_id: Option<String>,
        token: String,
        owner_id: AccountId,
        amount: U128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> Promise {
        require!(
            !self.omni_tokens.contains(&token)
                || self.acl_has_any_role(
                    vec![Role::OmniDepositer.into(), Role::DAO.into()],
                    env::predecessor_account_id()
                ),
            "omni token deposit requires OmniDepositer role"
        );

        if let Some(deposit_id) = deposit_id {
            require!(self.deposits.insert(deposit_id), "deposit already exists");
        }

        require!(
            env::attached_deposit() >= self.bridge_token_storage_deposit_required,
            "not enough deposit attached for token storage_deposit"
        );
        require!(self.tokens.contains(&token), "token does not exist");

        let token_id = Self::token_id(token);

        if let Some(msg) = msg {
            require!(
                gas_left()
                    > POA_TOKEN_FT_DEPOSIT_GAS.saturating_add(POA_TOKEN_FT_TRANSFER_CALL_MIN_GAS),
                "insufficient gas"
            );
            ext_poa_fungible_token::ext(token_id.clone())
                .with_attached_deposit(env::attached_deposit())
                .with_static_gas(POA_TOKEN_FT_DEPOSIT_GAS)
                .ft_deposit(env::current_account_id(), amount, None)
                .then(
                    ext_ft_core::ext(token_id)
                        .with_attached_deposit(NearToken::from_yoctonear(1))
                        .ft_transfer_call(owner_id, amount, memo, msg),
                )
        } else {
            ext_poa_fungible_token::ext(token_id)
                .with_attached_deposit(env::attached_deposit())
                .with_static_gas(POA_TOKEN_FT_DEPOSIT_GAS)
                .ft_deposit(owner_id, amount, memo)
        }
    }

    #[pause]
    #[access_control_any(roles(Role::DAO, Role::TokenWithdrawer))]
    fn ft_withdraw(&mut self, withdrawal_id: String, withdrawal: Withdrawal) {
        require!(
            self.withdrawals
                .insert(withdrawal_id.clone(), withdrawal.clone())
                .is_none(),
            "withdrawal already exists"
        );
        FactoryEvent::FtWithdraw {
            withdrawal_id: &withdrawal_id,
            withdrawal: &withdrawal,
        }
        .emit();
    }

    #[pause]
    #[access_control_any(roles(Role::DAO, Role::TokenWithdrawer))]
    fn ft_update_withdraw(
        &mut self,
        transfer_id: String,
        prev_payload_hash: Base64VecU8,
        new_payload_hash: Base64VecU8,
        metadata: String,
    ) {
        let withdrawal = self
            .withdrawals
            .get_mut(&transfer_id)
            .unwrap_or_else(|| panic!("withdrawal not found"));

        require!(
            withdrawal.payload_hash == prev_payload_hash,
            "payload hash mismatch"
        );

        withdrawal.payload_hash = new_payload_hash;
        withdrawal.metadata = metadata;

        FactoryEvent::FtUpdateWithdraw {
            transfer_id: &transfer_id,
            prev_payload_hash: &prev_payload_hash,
            new_payload_hash: &withdrawal.payload_hash,
            metadata: &withdrawal.metadata,
        }
        .emit();
    }

    #[pause]
    #[access_control_any(roles(Role::DAO))]
    fn remove_withdraws(&mut self, withdrawals: Vec<String>) {
        for id in withdrawals {
            self.withdrawals.remove(&id);
        }
    }

    #[pause]
    #[access_control_any(roles(Role::DAO))]
    fn remove_deposits(&mut self, deposits: Vec<String>) {
        for id in deposits {
            self.deposits.remove(&id);
        }
    }

    fn get_withdraw(&self, withdrawal_id: String) -> Option<&Withdrawal> {
        self.withdrawals.get(&withdrawal_id)
    }

    fn tokens(&self) -> HashMap<String, AccountId> {
        self.tokens
            .iter()
            .cloned()
            .map(|token| {
                let account_id = Self::token_id(&token);
                (token, account_id)
            })
            .collect()
    }

    #[pause]
    #[access_control_any(roles(Role::DAO))]
    fn add_omni_tokens(&mut self, tokens: Vec<String>) {
        for token in tokens {
            self.omni_tokens.insert(token);
        }
    }

    #[pause]
    #[access_control_any(roles(Role::DAO))]
    fn remove_omni_tokens(&mut self, tokens: Vec<String>) {
        for token in tokens {
            self.omni_tokens.remove(&token);
        }
    }

    fn get_omni_tokens(&self) -> Vec<String> {
        self.omni_tokens.iter().cloned().collect()
    }
}

impl Contract {
    #[track_caller]
    #[inline]
    fn token_id(token: impl AsRef<str>) -> AccountId {
        let token = token.as_ref();
        require!(!token.contains('.'), "invalid token name");
        format!("{token}.{}", env::current_account_id())
            .parse()
            .unwrap_or_else(|e| panic!("{e}"))
    }
}

#[near]
impl FullAccessKeys for Contract {
    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn add_full_access_key(&mut self, public_key: PublicKey) -> Promise {
        assert_one_yocto();
        Promise::new(env::current_account_id()).add_full_access_key(public_key)
    }

    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn delete_key(&mut self, public_key: PublicKey) -> Promise {
        assert_one_yocto();
        Promise::new(env::current_account_id()).delete_key(public_key)
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "::near_sdk::borsh")]
enum Prefix {
    Tokens,
    Deposits,
    Withdrawals,
    OmniTokens,
}
