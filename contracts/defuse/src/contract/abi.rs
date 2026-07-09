#![allow(clippy::used_underscore_binding)]

use defuse_core::{
    intents::{DefuseIntents, Intent},
    payload::{DefusePayload, nep413::Nep413DefuseMessage},
};
use near_sdk::near;
use serde::{Deserialize, Serialize};

use super::{Contract, ContractExt};

#[near]
impl Contract {
    pub fn __abi_helper(_types: AbiHelper) {}
}

#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Serialize, Deserialize)]
pub struct AbiHelper {
    pub intent: Intent,
    pub payload: AbiPayloadHelper,
}

#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Serialize, Deserialize)]
pub struct AbiPayloadHelper {
    pub nep413: Nep413DefuseMessage<DefuseIntents>,
    pub defuse: DefusePayload<DefuseIntents>,
}
