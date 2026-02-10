use core::fmt::{self, Display};

use near_sdk::near;

use crate::SigningStandard;

/// Always rejects the signature.
///
/// This can be useful to deploy "1-of-M multisig"/"fan-out" wallet, where
/// extensions are defined at the initialization stage (i.e. state_init).
/// So only extensions can execute requests via `w_execute_extension()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoSign;

impl SigningStandard for NoSign {
    type PublicKey = NoPublicKey;

    fn verify(_msg: &[u8], _public_key: &Self::PublicKey, _signature: &str) -> bool {
        false
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoPublicKey;

impl Display for NoPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no-sign")
    }
}
