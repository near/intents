use near_sdk::{AccountId, GlobalContractId, near};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyConfig {
    pub owner: AccountId,
    pub oneshot_condvar_global_id: GlobalContractId,
    pub escrow_swap_contract_id: GlobalContractId,
    pub auth_contract: AccountId,
    pub notifier: AccountId,
}
