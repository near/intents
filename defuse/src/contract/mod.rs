#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
mod abi;
mod accounts;
mod admin;
pub mod config;
mod events;
mod fees;
mod intents;
mod state;
mod tokens;
mod upgrade;

use core::iter;

use defuse_core::Result;

use events::PostponedMtBurnEvents;
use impl_tools::autoimpl;
use near_plugins::{AccessControlRole, AccessControllable, Pausable, access_control};
use near_sdk::{
    BorshStorageKey, PanicOnDefault, borsh::BorshDeserialize, near, require, store::LookupSet,
};

use crate::Defuse;

use self::{
    accounts::Accounts,
    config::{DefuseConfig, RolesConfig},
    state::ContractState,
};

#[near(serializers = [json])]
#[derive(AccessControlRole, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    DAO,

    FeesManager,
    RelayerKeysManager,

    UnrestrictedWithdrawer,

    PauseManager,
    Upgrader,
    UnpauseManager,
}

#[access_control(role_type(Role))]
#[derive(Pausable, PanicOnDefault)]
#[pausable(
    pause_roles(Role::DAO, Role::PauseManager),
    unpause_roles(Role::DAO, Role::UnpauseManager)
)]
#[near(
    contract_state,
    contract_metadata(
        standard(standard = "dip4", version = "0.1.0"),
        standard(standard = "nep245", version = "1.0.0"),
    )
)]
#[autoimpl(Deref using self.state)]
#[autoimpl(DerefMut using self.state)]
pub struct Contract {
    accounts: Accounts,
    state: ContractState,

    relayer_keys: LookupSet<near_sdk::PublicKey>,

    #[borsh(skip)]
    postponed_burns: PostponedMtBurnEvents,
}

#[near]
impl Contract {
    #[must_use]
    #[init]
    pub fn new(config: DefuseConfig) -> Self {
        let mut contract = Self {
            accounts: Accounts::new(Prefix::Accounts),
            state: ContractState::new(Prefix::State, config.wnear_id, config.fees),
            relayer_keys: LookupSet::new(Prefix::RelayerKeys),
            postponed_burns: PostponedMtBurnEvents::new(),
        };
        contract.init_acl(config.roles);
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
impl Defuse for Contract {}

#[derive(BorshStorageKey)]
#[near(serializers = [borsh])]
enum Prefix {
    Accounts,
    State,
    RelayerKeys,
}
