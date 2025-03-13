use near_sdk::{Gas, env};

#[inline]
pub fn gas_left() -> Gas {
    env::prepaid_gas().saturating_sub(env::used_gas())
}
