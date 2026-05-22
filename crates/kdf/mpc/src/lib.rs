mod ckd;
// #[cfg(feature = "ed25519")]
// mod ed25519;
// #[cfg(feature = "secp256k1")]
// mod secp256k1;
mod tweak;

pub use self::{ckd::*, tweak::*};

use defuse_kdf::digest::Digest;
use near_account_id::AccountIdRef;
use sha3::{Digest as _, Sha3_256};

/// See <https://github.com/near/mpc/blob/f07b9145b17e2372be768aa67a2106be9989a7d7/crates/near-mpc-crypto-types/src/kdf.rs#L25-L39>
fn derive_from_path(
    hasher: Sha3_256,
    predecessor_id: impl AsRef<AccountIdRef>,
) -> Digest<Sha3_256> {
    let hasher = hasher
        .chain_update(predecessor_id.as_ref().as_bytes())
        .chain_update(",");

    Digest::new(hasher)
}
