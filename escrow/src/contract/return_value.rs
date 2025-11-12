use near_sdk::{Gas, GasWeight, NearToken, Promise};

pub const RETURN_VALUE_GAS: Gas = Gas::from_ggas(500);

// optimized implementation without `#[near]` macro
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn return_value() {
    if let Some(input) = ::near_sdk::env::input() {
        ::near_sdk::env::value_return(&input);
    }
}

pub trait ReturnValueExt: Sized {
    fn return_value(self, value: Vec<u8>) -> Self;
}

impl ReturnValueExt for Promise {
    fn return_value(self, value: Vec<u8>) -> Self {
        self.function_call_weight(
            "return_value".to_string(),
            value,
            NearToken::from_yoctonear(0),
            RETURN_VALUE_GAS,
            GasWeight(0),
        )
    }
}
