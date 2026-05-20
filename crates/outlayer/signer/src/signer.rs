use hkdf::Hkdf;
use sha3::Sha3_512;

#[cfg_attr(feature = "zeroize", derive(::zeroize::ZeroizeOnDrop))]
#[derive(Clone, PartialEq, Eq)]
pub struct InMemorySigner {
    #[cfg(feature = "ed25519")]
    pub(crate) ed25519_master_sk: defuse_kdf::ed25519_dalek::SigningKey,
    #[cfg(feature = "secp256k1")]
    pub(crate) secp256k1_master_sk: defuse_kdf::k256::ecdsa::SigningKey,
}

impl InMemorySigner {
    /// Construct from a seed, i.e. input key material with _not necessarily_
    /// uniformly distributed entropy.
    ///
    /// NOTE: `seed` is passed by reference, and the referenced value is
    /// recommended to be zeroized afterwards.
    pub fn from_seed(seed: &[u8]) -> Self {
        const SALT: &[u8] = b"outlayer v0.1.0 signer seed:";

        let hk = Hkdf::<Sha3_512>::new(Some(SALT), seed);

        Self {
            #[cfg(feature = "ed25519")]
            ed25519_master_sk: {
                const INFO: &[u8] = b"ed25519/root_sk";

                let mut sk = [0u8; defuse_kdf::ed25519_dalek::SECRET_KEY_LENGTH];
                hk.expand(INFO, &mut sk).expect("HKDF: ed25519");
                defuse_kdf::ed25519_dalek::SigningKey::from_bytes(&sk)
            },
            #[cfg(feature = "secp256k1")]
            secp256k1_master_sk: {
                const INFO: &[u8] = b"secp256k1/root_sk";

                let mut sk = [0u8; 32];
                hk.expand(INFO, &mut sk).expect("HKDF: secp256k1");
                defuse_kdf::k256::ecdsa::SigningKey::from_bytes(&sk.into())
                    .expect("secp256k1: derived scalar is zero or less than curve order")
            },
        }
    }
}
