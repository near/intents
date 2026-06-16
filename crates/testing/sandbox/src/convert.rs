use near_kit::{
    CryptoHash, DeterministicAccountStateInit, DeterministicAccountStateInitV1,
    GlobalContractIdentifier,
};
// use near_sdk::{
//     GlobalContractId,
//     state_init::{StateInit, StateInitV1},
// };

// TODO: should there be any support for it in near sdk?
// pub trait ConvertInto<T> {
//     fn convert_into(self) -> T;
// }

// impl ConvertInto<DeterministicAccountStateInit> for StateInit {
//     #[inline]
//     fn convert_into(self) -> DeterministicAccountStateInit {
//         match self {
//             StateInit::V1(StateInitV1 { code, data }) => {
//                 DeterministicAccountStateInit::V1(DeterministicAccountStateInitV1 {
//                     code: match code {
//                         GlobalContractId::CodeHash(hash) => {
//                             GlobalContractIdentifier::CodeHash(CryptoHash::from_bytes(hash.into()))
//                         }
//                         GlobalContractId::AccountId(account) => {
//                             GlobalContractIdentifier::AccountId(account)
//                         }
//                     },
//                     data,
//                 })
//             }
//         }
//     }
// }

// impl ConvertInto<GlobalContractId> for GlobalContractIdentifier {
//     fn convert_into(self) -> GlobalContractId {
//         match self {
//             GlobalContractIdentifier::AccountId(account_id) => {
//                 GlobalContractId::AccountId(account_id)
//             }
//             GlobalContractIdentifier::CodeHash(code_hash) => {
//                 GlobalContractId::CodeHash(code_hash.as_bytes().into())
//             }
//         }
//     }
// }

// impl ConvertInto<GlobalContractIdentifier> for GlobalContractId {
//     fn convert_into(self) -> GlobalContractIdentifier {
//         match self {
//             GlobalContractId::AccountId(account_id) => {
//                 GlobalContractIdentifier::AccountId(account_id)
//             }
//             GlobalContractId::CodeHash(code_hash) => {
//                 GlobalContractIdentifier::CodeHash(CryptoHash::from_bytes(code_hash.into()))
//             }
//         }
//     }
// }
