use core::iter;
use std::collections::{HashMap, HashSet};

use defuse_admin_utils::full_access_keys::FullAccessKeys;
use defuse_controller::ControllerUpgradable;
use defuse_near_utils::{CURRENT_ACCOUNT_ID, UnwrapOrPanicError, gas_left, method_name};
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

use crate::PoaFactory;

const STATE_MIGRATE_FUNCTION: &str = method_name!(Contract::state_migrate);
const STATE_MIGRATE_DEFAULT_GAS: Gas = Gas::from_tgas(5);

#[cfg(not(clippy))]
const POA_TOKEN_WASM: &[u8] = include_bytes!(std::env!(
    "POA_TOKEN_WASM",
    "Set ${POA_TOKEN_WASM} to be the path of the PoA token binary",
));
#[cfg(clippy)]
const POA_TOKEN_WASM: &[u8] = b"";

const POA_TOKEN_INIT_BALANCE: NearToken = NearToken::from_near(5);
const POA_TOKEN_NEW_GAS: Gas = Gas::from_tgas(10);
const POA_TOKEN_FT_DEPOSIT_GAS: Gas = Gas::from_tgas(10);
/// Copied from `near_contract_standards::fungible_token::core_impl::GAS_FOR_FT_TRANSFER_CALL`
const POA_TOKEN_FT_TRANSFER_CALL_MIN_GAS: Gas = Gas::from_tgas(30);

const POA_TOKEN_UPRADE_GAS: Gas = Gas::from_tgas(10);

#[derive(AccessControlRole, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[near(serializers = [json])]
pub enum Role {
    DAO,
    TokenDeployer,
    TokenDepositer,
    PauseManager,
    UnpauseManager,
}

#[near(contract_state)]
#[derive(Pausable, PanicOnDefault)]
#[access_control(role_type(Role))]
#[pausable(
    pause_roles(Role::DAO, Role::PauseManager),
    unpause_roles(Role::DAO, Role::UnpauseManager)
)]
pub struct Contract {
    tokens: IterableSet<String>,
    bridge_token_storage_deposit_required: NearToken,
}

#[near]
impl Contract {
    #[must_use]
    #[init]
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

    #[pause]
    #[access_control_any(roles(Role::DAO, Role::TokenDeployer))]
    #[payable]
    pub fn upgrade_token(&mut self, token: String, self_public_key: PublicKey) -> Promise {
        // FIXME: Check what conditions for atomicity, safety and repeatability are needed
        Promise::new(Self::token_id(token))
            .function_call(
                "add_full_access_key".to_string(),
                serde_json::to_vec(&json!({
                    "public_key": self_public_key,
                }))
                .unwrap_or_panic_display(),
                NearToken::from_near(0),
                POA_TOKEN_UPRADE_GAS,
            )
            .deploy_contract(POA_TOKEN_WASM.to_vec())
            .function_call(
                "upgrade_to_versioned".to_string(),
                serde_json::to_vec(&json!({})).unwrap_or_panic_display(),
                NearToken::from_yoctonear(1), // FIXME: amount
                POA_TOKEN_UPRADE_GAS,
            )
            .function_call(
                "delete_key".to_string(),
                serde_json::to_vec(&json!({
                    "public_key": self_public_key,
                }))
                .unwrap_or_panic_display(),
                NearToken::from_near(0),
                POA_TOKEN_UPRADE_GAS,
            )
    }
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
                .unwrap_or_panic_display(),
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
    #[access_control_any(roles(Role::DAO, Role::TokenDepositer))]
    #[payable]
    fn ft_deposit(
        &mut self,
        token: String,
        owner_id: AccountId,
        amount: U128,
        msg: Option<String>,
        memo: Option<String>,
    ) -> Promise {
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
                .ft_deposit(CURRENT_ACCOUNT_ID.clone(), amount, None)
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

    fn tokens(&self) -> HashMap<String, AccountId> {
        self.tokens
            .iter()
            .map(|token| {
                let account_id = Self::token_id(token);
                (token.to_string(), account_id)
            })
            .collect()
    }
}

impl Contract {
    #[track_caller]
    #[inline]
    fn token_id(token: impl AsRef<str>) -> AccountId {
        let token = token.as_ref();
        require!(!token.contains('.'), "invalid token name");
        format!("{token}.{}", *CURRENT_ACCOUNT_ID)
            .parse()
            .unwrap_or_panic_display()
    }
}

#[near]
impl FullAccessKeys for Contract {
    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn add_full_access_key(&mut self, public_key: PublicKey) -> Promise {
        assert_one_yocto();
        Promise::new(CURRENT_ACCOUNT_ID.clone()).add_full_access_key(public_key)
    }

    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn delete_key(&mut self, public_key: PublicKey) -> Promise {
        assert_one_yocto();
        Promise::new(CURRENT_ACCOUNT_ID.clone()).delete_key(public_key)
    }
}

#[near]
impl ControllerUpgradable for Contract {
    #[access_control_any(roles(Role::DAO, Role::TokenDeployer))]
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
    Tokens,
}
