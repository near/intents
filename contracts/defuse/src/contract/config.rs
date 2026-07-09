use std::collections::{HashMap, HashSet};

use defuse_core::fees::FeesConfig;
use near_sdk::AccountId;
use serde::{Deserialize, Serialize};

use super::Role;

#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefuseConfig {
    pub wnear_id: AccountId,
    pub fees: FeesConfig,
    #[serde(default)]
    pub roles: RolesConfig,
}

#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RolesConfig {
    #[serde(default)]
    pub super_admins: HashSet<AccountId>,
    #[serde(default)]
    pub admins: HashMap<Role, HashSet<AccountId>>,
    #[serde(default)]
    pub grantees: HashMap<Role, HashSet<AccountId>>,
}
