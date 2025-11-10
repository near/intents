#[cfg(feature = "nep141")]
mod nep141;
#[cfg(feature = "nep245")]
mod nep245;

// TODO: assert any(feature = "nep141", feature = "nep245")

// use defuse_token_id::TokenId;
use near_sdk::{AccountId, Gas, Promise};

pub trait Token {
    fn send(
        self,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<Gas>,
        unused_gas: bool,
    ) -> Promise;

    // Returns actually transferred amount of a single token.
    fn resolve(result_idx: u64, amount: u128, is_call: bool) -> u128;
}

// pub trait TokenType {
//     fn parse_transfer_ok(&self, data: &[u8]) -> bool;
//     fn parse_transfer_call_ok(&self, data: &[u8]) -> u128;

//     fn transfer_call_failed_refund(&self) -> bool;

//     fn resolve_transfer(&self, result_idx: u64, amount: u128, is_call: bool) -> u128 {
//         match env::promise_result(result_idx) {
//             PromiseResult::Successful(data) => {
//                 if is_call {
//                     self.parse_transfer_call_ok(&data).min(amount)
//                 } else if self.parse_transfer_ok(&data) {
//                     amount
//                 } else {
//                     0
//                 }
//             }
//             PromiseResult::Failed => {
//                 if is_call {
//                     // do not refund on failed `mt_transfer_call` due to
//                     // NEP-245 vulnerability: `mt_resolve_transfer` fails to
//                     // read result of `mt_on_transfer` due to insufficient gas
//                     amount
//                 } else {
//                     0
//                 }
//             }
//         }
//     }
// }
