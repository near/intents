mod admin;
mod tokens;
mod upgrade;
mod utils;

use core::iter;

use defuse_escrow_swap::ext_escrow;
use defuse_oneshot_condvar::{CondVarContext, ext_oneshot_condvar};
use near_plugins::{AccessControlRole, AccessControllable, access_control, access_control_any};
use near_sdk::{
    AccountId, CryptoHash, Gas, NearToken, PanicOnDefault, Promise, env, near, require,
    state_init::{StateInit, StateInitV1},
};

use crate::message::EscrowParams;
use crate::state::{ProxyConfig, RolesConfig};
use crate::{EscrowProxy, Role, RoleFlags};

#[access_control(role_type(Role))]
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    config: ProxyConfig,
}

#[near]
impl Contract {
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(roles: RolesConfig, config: ProxyConfig) -> Contract {
        let mut contract = Self { config };
        contract.init_acl(roles);
        contract
    }

    fn init_acl(&mut self, roles: RolesConfig) {
        let mut acl = self.acl_get_or_init();
        require!(
            roles
                .super_admins
                .into_iter()
                .all(|super_admin| acl.add_super_admin_unchecked(&super_admin))
                && roles
                    .admins
                    .into_iter()
                    .flat_map(|(role, admins)| iter::repeat(role).zip(admins))
                    .all(|(role, admin)| acl.add_admin_unchecked(role, &admin))
                && roles
                    .grantees
                    .into_iter()
                    .flat_map(|(role, grantees)| iter::repeat(role).zip(grantees))
                    .all(|(role, grantee)| acl.grant_role_unchecked(role, &grantee)),
            "failed to set roles"
        );
    }
}

#[near]
impl EscrowProxy for Contract {
    fn config(&self) -> &ProxyConfig {
        &self.config
    }

    fn context_hash(&self, context: CondVarContext<'static>) -> CryptoHash {
        context.hash()
    }
}

#[near]
impl Contract {
    #[access_control_any(roles(Role::DAO, Role::Canceller))]
    pub fn cancel_escrow(&self, params: EscrowParams) -> Promise {
        let escrow_address = {
            let raw_state = defuse_escrow_swap::ContractStorage::init_state(&params)
                .unwrap_or_else(|e| env::panic_str(&format!("Invalid escrow params: {e}")));
            let state_init = StateInit::V1(StateInitV1 {
                code: self.config.escrow_swap_contract_id.clone(),
                data: raw_state,
            });
            state_init.derive_account_id()
        };
        ext_escrow::ext(escrow_address)
            .with_static_gas(Gas::from_tgas(50))
            .es_close(params)
    }

    #[access_control_any(roles(Role::DAO, Role::Canceller))]
    pub fn authorize(&self, condvar_id: AccountId) -> Promise {
        ext_oneshot_condvar::ext(condvar_id)
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(Gas::from_tgas(50))
            .cv_notify_one()
    }
}
