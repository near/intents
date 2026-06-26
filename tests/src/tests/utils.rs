use std::fmt::Display;

use defuse_fees::Pips;
use defuse_randomness::{Rng, RngExt};
use defuse_test_utils::random::rng;
use rstest::rstest;

#[rstest]
fn pips_borsch_serialization_back_and_forth(mut rng: impl Rng) {
    let pip_val = rng.random_range::<u32, _>(0..=Pips::MAX.as_pips());

    let pip = Pips::from_pips(pip_val).unwrap();
    let serialized = borsh::to_vec(&pip).unwrap();
    let deserialized: Pips = borsh::from_slice(&serialized).unwrap();
    assert_eq!(deserialized, pip);
}

#[rstest]
#[trace]
#[case(&[206, 137, 2, 0], 166_350)]
#[trace]
#[case(&[116, 38, 2, 0], 140_916)]
#[trace]
#[case(&[3, 186, 2, 0], 178_691)]
#[trace]
#[case(&[199, 66, 12, 0], 803_527)]
#[trace]
#[case(&[73, 131, 13, 0], 885_577)]
#[trace]
#[case(&[64, 66, 15, 0], 1_000_000)]
#[trace]
#[case(&[0, 0, 0, 0], 0)]
fn pip_borsch_deserialization_selected_values(#[case] serialized: &[u8], #[case] pips: u32) {
    let deserialized: Pips = borsh::from_slice(serialized).unwrap();
    assert_eq!(deserialized, Pips::from_pips(pips).unwrap());
}

/// Assert that collection `a` contains collection `b`.
/// Checks that all elements in `b` are present in `a`.
///
/// # Examples
/// ```ignore
/// assert_a_contains_b(a: all_logs, b: [expected_event1, expected_event2]);
/// ```
#[track_caller]
pub fn assert_a_contains_b(
    a: impl IntoIterator<Item: Display>,
    b: impl IntoIterator<Item: Display>,
) {
    let a: Vec<String> = a.into_iter().map(|v| v.to_string()).collect();
    let b: Vec<String> = b.into_iter().map(|v| v.to_string()).collect();

    for expected_event in &b {
        assert!(
            a.contains(expected_event),
            "\n\nExpected event not found in 'a':\n{expected_event}\n\nActual event logs in 'a':\n{a:#?}\n",
        );
    }
}
