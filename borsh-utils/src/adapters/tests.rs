use super::*;
use std::fmt::Debug;

// Helper roundtrip
#[track_caller]
pub fn roundtrip_as<T, As>(orig: &T)
where
    As: BorshSerializeAs<T> + BorshDeserializeAs<T>,
    T: PartialEq + Debug,
{
    let mut buf = Vec::new();
    As::serialize_as(orig, &mut buf).expect("serialize_as");
    let deserialized = As::deserialize_as(&mut buf.as_slice()).expect("deserialize_as");
    assert_eq!(
        &deserialized, orig,
        "deserialized value differs from the original one"
    );
}

#[test]
fn same_identity() {
    roundtrip_as::<u32, Same>(&42);
}

#[test]
fn array() {
    roundtrip_as::<[u8; 3], [Same; 3]>(&[1, 2, 3]);
}

#[test]
fn tuple() {
    roundtrip_as::<(u8, u8), (Same, Same)>(&(10, 20));
}
