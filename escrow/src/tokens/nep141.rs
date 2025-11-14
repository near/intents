use defuse_token_id::{TokenIdType, nep141::Nep141TokenId};
use near_sdk::Gas;

use crate::tokens::TokenIdExt;

const FT_TRANSFER_GAS_MIN: Gas = Gas::from_tgas(15);
const FT_TRANSFER_GAS_DEFAULT: Gas = Gas::from_tgas(15);

const FT_TRANSFER_CALL_GAS_MIN: Gas = Gas::from_tgas(30);
const FT_TRANSFER_CALL_GAS_DEFAULT: Gas = Gas::from_tgas(50);

impl TokenIdExt for Nep141TokenId {
    #[inline]
    fn token_type(&self) -> TokenIdType {
        TokenIdType::Nep141
    }

    fn transfer_gas_min_default(&self, is_call: bool) -> (Gas, Gas) {
        if is_call {
            (FT_TRANSFER_CALL_GAS_MIN, FT_TRANSFER_CALL_GAS_DEFAULT)
        } else {
            (FT_TRANSFER_GAS_MIN, FT_TRANSFER_GAS_DEFAULT)
        }
    }
}
