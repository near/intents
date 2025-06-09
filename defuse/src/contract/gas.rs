use defuse_near_utils::UnwrapOrPanicError;
use near_sdk::Gas;

// These represent a linear model total_gas_cost = per_token*n + base,
// where `n` is the number of tokens in one mt_withdraw call.
// fitted from points {(1, 11.9), (2, 13.1), (3, 14.4)}
// The model fitted from the test at the time of writing is:  10.63+1.25n,
// with parameter errors:
// base:       10.63 +/- 0.062
// per_token:  1.25  +/- 0.028
// Values calculated in Mathematica using:
// lm = LinearModelFit[{{1, 11.9}, {2, 13.1}, {3, 14.4}}, {1, n}, n]
// lm["ParameterErrors"]
// The values below add 30% margin to the estimated values
const MT_RESOLVE_TRANSFER_GAS_PER_TOKEN: Gas = Gas::from_tgas(14);
const MT_RESOLVE_TRANSFER_BASE_GAS: Gas = Gas::from_tgas(5);

#[must_use]
pub fn total_mt_withdraw_gas(token_count: usize) -> Gas {
    let token_count: u64 = token_count.try_into().unwrap_or_panic_display();

    MT_RESOLVE_TRANSFER_BASE_GAS
        .saturating_add(MT_RESOLVE_TRANSFER_GAS_PER_TOKEN.saturating_mul(token_count))
}
