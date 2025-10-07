#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
mod abi;
mod accounts;
mod admin;
pub mod config;
mod events;
mod fees;
mod intents;
mod salts;
mod state;
mod tokens;
mod upgrade;
mod versioned;

use core::iter;

use defuse_borsh_utils::adapters::As;
use defuse_core::Result;
use events::PostponedMtBurnEvents;
use impl_tools::autoimpl;
use near_plugins::{AccessControlRole, AccessControllable, Pausable, access_control};
use near_sdk::{
    BorshStorageKey, PanicOnDefault,
    borsh::{BorshDeserialize, BorshSerialize},
    near, require,
    store::LookupSet,
};
use versioned::MaybeVersionedContractEntry;

use crate::Defuse;

use self::{
    accounts::Accounts,
    config::{DefuseConfig, RolesConfig},
    state::{ContractState, MaybeVersionedStateEntry},
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

    UnrestrictedAccountLocker,
    UnrestrictedAccountUnlocker,

    SaltManager,
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
#[autoimpl(Deref using self.contract)]
#[autoimpl(DerefMut using self.contract)]
pub struct ContractEntry {
    #[borsh(
        deserialize_with = "As::<MaybeVersionedContractEntry>::deserialize",
        serialize_with = "As::<MaybeVersionedContractEntry>::serialize"
    )]
    contract: Contract,
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
#[borsh(crate = "::near_sdk::borsh")]
#[autoimpl(Deref using self.state)]
#[autoimpl(DerefMut using self.state)]
struct Contract {
    accounts: Accounts,

    #[borsh(
        deserialize_with = "As::<MaybeVersionedStateEntry>::deserialize",
        serialize_with = "As::<MaybeVersionedStateEntry>::serialize"
    )]
    state: ContractState,

    relayer_keys: LookupSet<near_sdk::PublicKey>,

    #[borsh(skip)]
    postponed_burns: PostponedMtBurnEvents,
}

#[near]
impl ContractEntry {
    #[must_use]
    #[init]
    #[allow(clippy::use_self)] // Clippy seems to not play well with near-sdk, or there is a bug in clippy - seen in shared security analysis
    pub fn new(config: DefuseConfig) -> Self {
        let mut contract = Self {
            contract: Contract {
                accounts: Accounts::new(Prefix::Accounts),
                state: ContractState::new(Prefix::State, config.wnear_id, config.fees),
                relayer_keys: LookupSet::new(Prefix::RelayerKeys),
                postponed_burns: PostponedMtBurnEvents::new(),
            },
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
impl Defuse for ContractEntry {}

#[derive(BorshStorageKey)]
#[near(serializers = [borsh])]
enum Prefix {
    Accounts,
    State,
    RelayerKeys,
}
