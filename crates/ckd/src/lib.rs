use blstrs::{Bls12, G1Affine, G1Projective, G2Affine, G2Projective, Scalar, pairing};
use defuse_kdf_mpc::{ckd::CkdSchema, kdf::Schema};
use pairing::{
    MillerLoopResult, MultiMillerLoop,
    group::{Curve, Group, ff::Field, prime::PrimeCurveAffine},
};

use near_account_id::AccountIdRef;
use rand_core::CryptoRng;
// use zeroize::ZeroizeOnDrop;

// TODO
// #[derive(ZeroizeOnDrop)]
pub struct Ckd {
    private_key: Scalar,
}

const NEAR_CKD_DOMAIN: &[u8] = b"NEAR BLS12381G1_XMD:SHA-256_SSWU_RO_";

// TODO
// let app_id = defuse_kdf_mpc::ckd(predecessor_id).derive_path(path);

impl Ckd {
    // TODO: implement Distribution?
    pub fn random(rng: impl CryptoRng) -> (Self, G1Projective, G2Projective) {
        // TODO: legacy adaptor
        Self::from_scalar(Scalar::random(todo!()))
    }

    fn from_scalar(x: Scalar) -> (Self, G1Projective, G2Projective) {
        let pk1 = G1Projective::generator() * x;
        let pk2 = G2Projective::generator() * x;

        (Self { private_key: x }, pk1, pk2)
    }

    // TODO: return size
    pub fn decrypt_and_verify(
        &self,
        big_y: G1Projective,
        big_c: G1Projective,
        mpc_public_key: G2Projective,
    ) -> Option<[u8; G1Affine::compressed_size()]> {
        let secret = self.decrypt(big_y, big_c);

        if !Self::verify_legacy(mpc_public_key, secret) {
            return None;
        }

        Some(secret.to_compressed())
    }

    fn decrypt(&self, big_y: G1Projective, big_c: G1Projective) -> G1Projective {
        // TODO: cfg(near)
        // env::bls12381_p1_sum() + env::bls12381_g1_multiexp()
        big_c - big_y * self.private_key
    }

    fn verify_legacy(public_key: G2Projective, signature: G1Projective) -> bool {
        let element1 = signature.to_affine();
        if (!element1.is_on_curve() | !element1.is_torsion_free() | element1.is_identity()).into() {
            return false;
        }

        let element2 = public_key.to_affine();
        if (!element2.is_on_curve() | !element2.is_torsion_free() | element2.is_identity()).into() {
            return false;
        }

        let hash_input = [public_key.to_compressed().as_slice(), app_id].concat();
        let hash_point = G1Projective::hash_to_curve(&hash_input, NEAR_CKD_DOMAIN, b"").into();
        let base2 = G2Affine::generator();

        // TODO: cfg(near)
        pairing(&hash_point, &element2) == pairing(&element1, &base2)
    }

    fn verify_pv(
        app_id: [u8; 32],
        app_pk2: G2Affine,
        big_y: G1Affine,
        big_c: G1Affine,
        public_key: G2Affine,
    ) -> bool {
        let minus_g2 = -G2Affine::generator();

        let hash_point: G1Affine = G1Projective::hash_to_curve(
            &[public_key.to_compressed().as_slice(), app_id.as_slice()].concat(),
            NEAR_CKD_DOMAIN,
            b"",
        )
        .into();

        let ml = Bls12::multi_miller_loop(&[
            (&big_c, &minus_g2.into()),
            (&big_y, &app_pk2.into()),
            (&hash_point, &public_key.into()),
        ]);

        let res = ml.final_exponentiation();

        res.is_identity().into()
    }
}

pub struct CkdVerifier {
    app_id: [u8; 32],
}

impl CkdVerifier {
    pub fn new(predecessor_id: impl AsRef<AccountIdRef>, path: impl AsRef<[u8]>) -> Self {
        Self {
            app_id: defuse_kdf_mpc::ckd(predecessor_id).derive_path(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    // const MPC_PUBLIC_KEY;

    #[rstest]
    #[case(
        hex!(""),
    )]
    fn test(#[case] secret: [u8; 48]) {
        let (ckd, p1, p2) = Ckd::new();
        let secret = ckd
            .decrypt_and_verify(big_y, big_c, mpc_public_key)
            .expect("verification failed");

        assert_eq!(secret,)
    }
}
