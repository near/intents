use std::collections::BTreeMap;

use anyhow::Result;
use near_kit::{Final, GlobalContractId, Near};

use crate::{
    global_contract::GlobalContract, nep616::DeployDeterministicAccountExt,
    outcome::SuccessfulExecutionOutcome,
};

#[near_kit::contract]
pub trait MtReceiverStub {
    fn dummy_method(&self);
}

// pub trait MtReceiverStubExt {
//     /// Deploy MT receiver stub as a global contract (subaccount of self)
//     #[allow(clippy::use_self)]
//     async fn deploy_mt_receiver_stub_global(
//         &self,
//         name: impl AsRef<str>,
//         wasm: impl Into<Vec<u8>>,
//     ) -> Result<GlobalContractId>;

//     /// Deploy an instance using `DeterministicStateInit` with the given raw state.
//     /// Returns the deterministic account ID derived from the state.
//     async fn deploy_mt_receiver_stub_instance(
//         &self,
//         global_contract_id: GlobalContractId,
//         raw_state: BTreeMap<Vec<u8>, Vec<u8>>,
//     ) -> Result<AccountId>;

//     /// Deploy an instance and return the execution outcome for gas analysis.
//     async fn deploy_mt_receiver_stub_instance_raw(
//         &self,
//         global_contract_id: GlobalContractId,
//         raw_state: BTreeMap<Vec<u8>, Vec<u8>>,
//     ) -> Result<(AccountId, SuccessfulExecutionOutcome)>;
// }

// impl MtReceiverStubExt for Near {
//     async fn deploy_mt_receiver_stub_global(
//         &self,
//         name: impl AsRef<str>,
//         wasm: impl Into<Vec<u8>>,
//     ) -> Result<GlobalContractId> {
//         let account_id = self.account_id().sub_account(name.as_ref()).unwrap();
//         self.deploy_upgradable_global_contract(account_id, wasm, NearToken::from_near(100))
//             .await
//     }

//     async fn deploy_mt_receiver_stub_instance(
//         &self,
//         global_contract_id: GlobalContractId,
//         raw_state: BTreeMap<Vec<u8>, Vec<u8>>,
//     ) -> Result<AccountId> {
//         self.deploy_deterministic_account(global_contract_id, raw_state, NearToken::ZERO)
//             .await
//     }

//     async fn deploy_mt_receiver_stub_instance_raw(
//         &self,
//         global_contract_id: GlobalContractId,
//         raw_state: BTreeMap<Vec<u8>, Vec<u8>>,
//     ) -> Result<(AccountId, SuccessfulExecutionOutcome)> {
//         let si = DeterministicAccountStateInit::V1(DeterministicAccountStateInitV1 {
//             code: global_contract_id,
//             data: raw_state,
//         });
//         let account_id = si.derive_account_id();
//         let outcome = self
//             .state_init(si, NearToken::ZERO)
//             .send()
//             .wait_until(Final)
//             .await?
//             .try_into()?;
//         Ok((account_id, outcome))
//     }
// }
