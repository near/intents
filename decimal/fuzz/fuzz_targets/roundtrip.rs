#![no_main]

use defuse_decimal::UD128;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|d: UD128| {
    assert_eq!(d.to_string().parse::<UD128>().unwrap(), d);
});
