use defuse_borsh_utils::adapters::{As, TryFromInto};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
#[borsh(crate = "::near_sdk::borsh")]
struct MyInt(
    #[borsh(
        serialize_with = "As::<Option<TryFromInto<i64>>>::serialize",
        deserialize_with = "As::<Option<TryFromInto<i64>>>::deserialize"
    )]
    Option<i32>,
);

fn main() {
    let v = MyInt(Some(123));
    let serialized = borsh::to_vec(&v).unwrap();
    let deserialized: MyInt = borsh::from_slice(&serialized).unwrap();
    assert_eq!(deserialized, v);
}
