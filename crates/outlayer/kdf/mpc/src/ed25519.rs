use defuse_outlayer_kdf::crypto::ed25519::{Ed25519, Scalar};
use near_mpc_crypto_types::Tweak;

use crate::{NearMpcCurve, sealed::Sealed};

impl NearMpcCurve for Ed25519 {
    fn tweak(tweak: Tweak) -> Scalar {
        // See <https://github.com/near/mpc/blob/1f833a13f70addc34eb1cff704f93fec61e7f7eb/crates/contract/src/crypto_shared/kdf.rs#L36>
        Scalar::from_bytes_mod_order(tweak.as_bytes())
    }
}

impl Sealed for Ed25519 {}
