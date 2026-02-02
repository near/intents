use near_sdk::Gas;

// Covers StateInit (NEP-616) cost when deterministic account doesn't exist yet.
// Only accounts for deploying via Global Contract ref (NEP-591) with <770B storage
// which doesn't require storage staking. If you need to attach more GAS, utilize
// `AuthCall::min_gas` or `NotifyOnTransfer::min_gas` and provide storage deposit separately.
pub const STATE_INIT_GAS: Gas = Gas::from_tgas(10);
