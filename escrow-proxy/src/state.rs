use std::collections::{HashMap, HashSet};

use near_sdk::{AccountId, GlobalContractId, near};

use crate::Role;

/// Configuration for role-based access control
#[near(serializers = [json])]
#[derive(Debug, Clone, Default)]
pub struct RolesConfig {
    pub super_admins: HashSet<AccountId>,
    pub admins: HashMap<Role, HashSet<AccountId>>,
    pub grantees: HashMap<Role, HashSet<AccountId>>,
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyConfig {
    pub per_fill_contract_id: GlobalContractId,
    pub escrow_swap_contract_id: GlobalContractId,
    pub auth_contract: AccountId,
    pub auth_collee: AccountId,
}
