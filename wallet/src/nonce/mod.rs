mod seqno;
pub use self::seqno::*;

#[cfg(feature = "highload")]
mod highload;
#[cfg(feature = "highload")]
pub use self::highload::*;
pub trait Nonces {
    type Nonce;
    type Error;

    fn commit(&mut self, nonce: Self::Nonce) -> Result<(), Self::Error>;
}

// use defuse_borsh_utils::adapters::{As, TimestampSeconds};
// use defuse_deadline::Deadline;
// use near_sdk::{near, serde_with::base64::Base64};

// #[near(serializers = [borsh(use_discriminant = true), json])]
// #[serde(tag = "schema", rename_all = "snake_case")]
// #[derive(Debug, Clone, PartialEq, Eq)]
// #[repr(u8)]
// pub enum Nonce {
//     Seqno {
//         /// MUST be equal to the current seqno on the contract.
//         seqno: u32,

//         /// The deadline for this signed request.
//         #[cfg_attr(
//             all(feature = "abi", not(target_arch = "wasm32")),
//             borsh(
//                 serialize_with = "As::<TimestampSeconds<u32>>::serialize",
//                 deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
//                 schema(with_funcs(
//                     definitions = "As::<TimestampSeconds<u32>>::add_definitions_recursively",
//                     declaration = "As::<TimestampSeconds<u32>>::declaration",
//                 ))
//             )
//         )]
//         #[cfg_attr(
//             any(not(feature = "abi"), target_arch = "wasm32"),
//             borsh(
//                 serialize_with = "As::<TimestampSeconds<u32>>::serialize",
//                 deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
//             )
//         )]
//         valid_until: Deadline,
//     } = 0,
//     Epoch {
//         #[cfg_attr(
//             all(feature = "abi", not(target_arch = "wasm32")),
//             borsh(
//                 serialize_with = "As::<TimestampSeconds<u32>>::serialize",
//                 deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
//                 schema(with_funcs(
//                     definitions = "As::<TimestampSeconds<u32>>::add_definitions_recursively",
//                     declaration = "As::<TimestampSeconds<u32>>::declaration",
//                 ))
//             )
//         )]
//         #[cfg_attr(
//             any(not(feature = "abi"), target_arch = "wasm32"),
//             borsh(
//                 serialize_with = "As::<TimestampSeconds<u32>>::serialize",
//                 deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
//             )
//         )]
//         created_at: Deadline,

//         nonce: u32,
//     } = 1,
//     Custom {
//         #[cfg_attr(
//             all(feature = "abi", not(target_arch = "wasm32")),
//             schemars(with = "String")
//         )]
//         #[serde_as(as = "Base64")]
//         args: Vec<u8>,
//     } = u8::MAX, // TODO: do we need it?
// }

// 0, 1, 2, [4], [5], [4], 3, [5]
// yield_id: 4
