use std::collections::{HashMap, HashSet};

use defuse_core::fees::FeesConfig;
use near_sdk::{AccountId, near};

use super::Role;

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct DefuseConfig {
    pub wnear_id: AccountId,
    pub fees: FeesConfig,
    pub roles: RolesConfig,
}

#[near(serializers = [json])]
#[derive(Debug, Clone, Default)]
pub struct RolesConfig {
    pub super_admins: HashSet<AccountId>,
    pub admins: HashMap<Role, HashSet<AccountId>>,
    pub grantees: HashMap<Role, HashSet<AccountId>>,
}
