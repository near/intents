use defuse_core::{
    intents::{DefuseIntents, Intent},
    payload::{DefusePayload, nep413::Nep413DefuseMessage},
};
use near_sdk::near;

use super::{ContractEntry, ContractEntryExt};

#[near]
impl ContractEntry {
    pub fn __abi_helper(types: AbiHelper) {}
}

#[near(serializers = [json])]
pub struct AbiHelper {
    pub intent: Intent,
    pub payload: AbiPayloadHelper,
}

#[near(serializers = [json])]
pub struct AbiPayloadHelper {
    pub nep413: Nep413DefuseMessage<DefuseIntents>,
    pub defuse: DefusePayload<DefuseIntents>,
}
