use impl_tools::autoimpl;
use near_sdk::{near, store::LookupSet};

use crate::contract::{
    ContractStorage, accounts::Accounts, events::PostponedMtBurnEvents, state::ContractStateV0,
};

#[derive(Debug)]
#[autoimpl(Deref using self.state)]
#[autoimpl(DerefMut using self.state)]
#[near(serializers = [borsh])]
pub struct ContractStorageV0 {
    accounts: Accounts,

    state: ContractStateV0,

    relayer_keys: LookupSet<near_sdk::PublicKey>,

    #[borsh(skip)]
    postponed_burns: PostponedMtBurnEvents,
}

impl From<ContractStorageV0> for ContractStorage {
    fn from(
        ContractStorageV0 {
            accounts,
            state,
            relayer_keys,
            postponed_burns,
        }: ContractStorageV0,
    ) -> Self {
        Self {
            accounts,
            state: state.into(),
            relayer_keys,
            postponed_burns,
        }
    }
}

// #[near]
// impl Contract {
//     #[must_use]
//     #[init]
//     #[allow(clippy::use_self)] // Clippy seems to not play well with near-sdk, or there is a bug in clippy - seen in shared security analysis
//     pub fn new(config: DefuseConfig) -> Self {
//         let mut contract = Self {
//             accounts: Accounts::new(Prefix::Accounts),
//             state: ContractState::new(Prefix::State, config.wnear_id, config.fees),
//             relayer_keys: LookupSet::new(Prefix::RelayerKeys),
//             postponed_burns: PostponedMtBurnEvents::new(),
//         };
//         contract.init_acl(config.roles);
//         contract
//     }

//     fn init_acl(&mut self, roles: RolesConfig) {
//         let mut acl = self.acl_get_or_init();
//         require!(
//             roles
//                 .super_admins
//                 .into_iter()
//                 .all(|super_admin| acl.add_super_admin_unchecked(&super_admin))
//                 && roles
//                     .admins
//                     .into_iter()
//                     .flat_map(|(role, admins)| iter::repeat(role).zip(admins))
//                     .all(|(role, admin)| acl.add_admin_unchecked(role, &admin))
//                 && roles
//                     .grantees
//                     .into_iter()
//                     .flat_map(|(role, grantees)| iter::repeat(role).zip(grantees))
//                     .all(|(role, grantee)| acl.grant_role_unchecked(role, &grantee)),
//             "failed to set roles"
//         );
//     }
// }

// #[near]
// impl Defuse for Contract {}

// #[derive(BorshStorageKey)]
// #[near(serializers = [borsh])]
// enum Prefix {
//     Accounts,
//     State,
//     RelayerKeys,
// }
