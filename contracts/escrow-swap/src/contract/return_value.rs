use near_sdk::{Gas, GasWeight, NearToken, Promise};

const RETURN_VALUE_GAS: Gas = Gas::from_ggas(500);

// optimized implementation without `#[near]` macro
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn es_return_value() {
    if let Some(input) = ::near_sdk::env::input() {
        ::near_sdk::env::value_return(&input);
    }
}

pub trait ReturnValueExt: Sized {
    fn es_return_value(self, value: impl Into<Vec<u8>>) -> Self;
}

impl ReturnValueExt for Promise {
    fn es_return_value(self, value: impl Into<Vec<u8>>) -> Self {
        self.function_call_weight(
            "es_return_value",
            value,
            NearToken::ZERO,
            RETURN_VALUE_GAS,
            GasWeight(0),
        )
    }
}
