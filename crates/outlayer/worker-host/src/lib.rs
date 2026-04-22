use borsh::BorshSerialize;
use hex_literal::hex;
use hkdf::Hkdf;
use k256::SecretKey as Secp256k1SecretKey;
use near_account_id::AccountId;

pub mod crypto;

pub struct WorkerHost {
    app_id: AppId,
    secp256k1_root_sk: Secp256k1SecretKey, // TODO: zeroize?
}

impl WorkerHost {
    pub fn new(app_id: AppId, seed: &[u8]) -> Self {
        // no salt is needed, seed is already with high entropy
        let hk = Hkdf::<sha3::Sha3_512>::new(None, seed);

        Self {
            app_id,
            secp256k1_root_sk: {
                // TODO: SHA3-256 would have done the job in one round, too
                let mut sk = [0u8; 32];
                hk.expand(
                    b"secp256k1", // TODO
                    &mut sk,
                )
                .unwrap();
                Secp256k1SecretKey::from_bytes(&sk.into())
                    // TODO: handle zero
                    .unwrap()
            },
        }
    }
}

#[derive(BorshSerialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum AppId {
    NearAccount(AccountId) = 0,
}

impl AppId {
    // TODO: hash of something?
    const SALT: &[u8] = &hex!("0000000000000000000000000000000000000000000000000000000000000000");

    // TODO: maybe different per-curve tweaks?
    pub fn derive_tweak(&self, path: impl AsRef<[u8]>) -> [u8; 32] {
        let hk = Hkdf::<sha3::Sha3_256>::new(
            Some(Self::SALT), // TODO: is salt needed?
            // TODO: does it make sense to put app_id in ikm?
            &borsh::to_vec(self).unwrap(),
        );
        let mut tweak = [0u8; 32];
        hk.expand(path.as_ref(), &mut tweak).unwrap();
        tweak
    }
}
