use defuse_kdf::digest::Digest;
use near_account_id::AccountIdRef;
use sha3::{Digest as _, Sha3_256};

use crate::derive_from_path;

pub fn ckd(predecessor_id: impl AsRef<AccountIdRef>) -> Digest<Sha3_256> {
    // See <https://github.com/near/mpc/blob/f07b9145b17e2372be768aa67a2106be9989a7d7/crates/near-mpc-crypto-types/src/kdf.rs#L15-L23>
    const APP_ID_DERIVATION_PREFIX: &str = "near-mpc v0.1.0 app_id derivation:";

    thread_local! {
        // per-thread lazily-initialized hasher with pre-processed prefix
        static HASHER: Sha3_256 = Sha3_256::new_with_prefix(APP_ID_DERIVATION_PREFIX);
    }

    derive_from_path(HASHER.with(Clone::clone), predecessor_id)
}
