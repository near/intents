use defuse_token_id::{TokenIdType, nep245::Nep245TokenId};
use near_sdk::Gas;

use crate::tokens::TokenIdExt;

const MT_TRANSFER_GAS_MIN: Gas = Gas::from_tgas(15);
const MT_TRANSFER_GAS_DEFAULT: Gas = Gas::from_tgas(15);

const MT_TRANSFER_CALL_GAS_MIN: Gas = Gas::from_tgas(30);
const MT_TRANSFER_CALL_GAS_DEFAULT: Gas = Gas::from_tgas(50);

impl TokenIdExt for Nep245TokenId {
    #[inline]
    fn token_type(&self) -> TokenIdType {
        TokenIdType::Nep245
    }

    #[inline]
    fn transfer_gas_min_default(&self, is_call: bool) -> (Gas, Gas) {
        if is_call {
            (MT_TRANSFER_CALL_GAS_MIN, MT_TRANSFER_CALL_GAS_DEFAULT)
        } else {
            (MT_TRANSFER_GAS_MIN, MT_TRANSFER_GAS_DEFAULT)
        }
    }
}
