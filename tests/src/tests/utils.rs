use defuse::core::fees::Pips;
use near_sdk::borsh;
use randomness::Rng;
use rstest::rstest;
use test_utils::random::{make_seedable_rng, Seed};

#[rstest]
#[trace]
#[case(Seed::from_entropy())]
fn pips_borsch_serialization_back_and_forth(#[case] seed: Seed) {
    let pip_val = make_seedable_rng(seed).gen_range::<u32, _>(0..=Pips::MAX.as_pips());

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
