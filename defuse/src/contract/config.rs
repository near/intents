use std::collections::{HashMap, HashSet};

use defuse_core::fees::FeesConfig;
use near_sdk::{AccountId, near};

use super::Role;

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct DefuseConfig {
    pub wnear_id: AccountId,
    pub fees: FeesConfig,
    #[serde(default)]
    pub roles: RolesConfig,
}

#[near(serializers = [json])]
#[derive(Debug, Clone, Default)]
pub struct RolesConfig {
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub super_admins: HashSet<AccountId>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub admins: HashMap<Role, HashSet<AccountId>>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub grantees: HashMap<Role, HashSet<AccountId>>,
}
