use defuse_outlayer_kdf::crypto::secp256k1::{Secp256k1, k256::NonZeroScalar};
use near_mpc_crypto_types::Tweak;

use crate::{NearMpcCurve, sealed::Sealed};

impl NearMpcCurve for Secp256k1 {
    fn tweak(tweak: Tweak) -> NonZeroScalar {
        // See <https://github.com/near/mpc/blob/1f833a13f70addc34eb1cff704f93fec61e7f7eb/crates/contract/src/crypto_shared/kdf.rs#L22>.
        NonZeroScalar::from_repr(tweak.as_bytes().into())
            .into_option()
            .expect("tweak is not on curve or zero")
    }
}

impl Sealed for Secp256k1 {}
