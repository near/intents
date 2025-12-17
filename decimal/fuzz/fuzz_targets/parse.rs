#![no_main]

use defuse_decimal::UD128;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|s: &str| {
    let _ = s.parse::<UD128>();
});
