use borsh::BorshSerialize;
use curve25519_dalek::Scalar as Ed25519Scalar;
use hkdf::Hkdf;
use k256::SecretKey as Secp256k1SecretKey;
use near_account_id::AccountId;

pub mod crypto;

pub struct WorkerHost {
    app_id: AppId,
    secp256k1_root_sk: Secp256k1SecretKey, // TODO: zeroize?
    /// Ed25519 root signing scalar (uniformly distributed in `[1, L)`).
    ///
    /// Unlike RFC 8032 signing keys, this scalar is NOT clamped: it is
    /// used as-is for the additive non-hardened derivation scheme in
    /// [`crypto::ed25519`]. Clamping is only relevant for X25519 and
    /// does not affect the correctness or security of Ed25519 signing.
    ed25519_root_scalar: Ed25519Scalar, // TODO: zeroize?
    /// Ed25519 "hash prefix" — the domain separator mixed into the
    /// deterministic nonce `r = H(prefix || msg) mod L` during
    /// signing. Shared across every derived path; uniqueness of `r`
    /// per `(path, msg)` is instead ensured by `A = sk(path) * G`
    /// being included in the challenge `c = H(R || A || msg)`.
    ed25519_root_prefix: [u8; 32], // TODO: zeroize?
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
            ed25519_root_scalar: {
                // 64 bytes → wide modular reduction gives a uniform
                // scalar in `[0, L)` (`L ≈ 2^252`, so a naive 32-byte
                // reduction would introduce a measurable ~2^-4 bias).
                let mut wide = [0u8; 64];
                hk.expand(b"ed25519/scalar", &mut wide).unwrap();
                let scalar = Ed25519Scalar::from_bytes_mod_order_wide(&wide);
                // Zero has probability ≈ 2^-252; treat as unreachable.
                assert_ne!(scalar, Ed25519Scalar::ZERO, "ed25519 root scalar is zero");
                scalar
            },
            ed25519_root_prefix: {
                let mut prefix = [0u8; 32];
                hk.expand(b"ed25519/prefix", &mut prefix).unwrap();
                prefix
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
