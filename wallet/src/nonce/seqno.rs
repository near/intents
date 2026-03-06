use defuse_borsh_utils::adapters::{As, TimestampSeconds};
use defuse_deadline::Deadline;
use near_sdk::near;
use thiserror::Error as ThisError;

use crate::Nonces;

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Seqno {
    /// Next valid seqno (i.e. nonce).
    pub seqno: u32,
}

impl Nonces for Seqno {
    type Nonce = SeqnoNonce;

    type Error = SeqnoError;

    fn commit(&mut self, nonce: Self::Nonce) -> Result<(), Self::Error> {
        // check seqno
        if nonce.seqno != self.seqno {
            return Err(SeqnoError::InvalidSeqno {
                got: nonce.seqno,
                expected: self.seqno,
            });
        }

        // check valid_until
        if nonce.valid_until.has_expired() {
            return Err(SeqnoError::Expired);
        }

        Ok(())
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SeqnoNonce {
    /// MUST be equal to the current seqno on the contract.
    pub seqno: u32,

    /// The deadline for this signed request.
    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        borsh(
            serialize_with = "As::<TimestampSeconds<u32>>::serialize",
            deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
            schema(with_funcs(
                definitions = "As::<TimestampSeconds<u32>>::add_definitions_recursively",
                declaration = "As::<TimestampSeconds<u32>>::declaration",
            ))
        )
    )]
    #[cfg_attr(
        any(not(feature = "abi"), target_arch = "wasm32"),
        borsh(
            serialize_with = "As::<TimestampSeconds<u32>>::serialize",
            deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
        )
    )]
    pub valid_until: Deadline,
}

#[derive(Debug, ThisError)]

pub enum SeqnoError {
    #[error("invalid seqno: {got}, expected: {expected}")]
    InvalidSeqno { got: u32, expected: u32 },

    #[error("signature expired")]
    Expired,
}
