pub use blstrs;

use blstrs::{Bls12, G1Affine, G1Projective, G2Affine, G2Projective, Scalar};
use defuse_kdf_mpc::kdf::Schema;
use defuse_rand_compat::RandCompat;
use near_account_id::AccountIdRef;
use pairing::{
    MillerLoopResult, MultiMillerLoop,
    group::{Curve, Group, ff::Field, prime::PrimeCurveAffine},
};
use rand_core::CryptoRng;

pub const SECRET_LEN: usize = G1Affine::compressed_size();
pub type Secret = [u8; SECRET_LEN];

// TODO: derive(ZeroizeOnDrop) once this PR is released:
// https://github.com/filecoin-project/blstrs/pull/66
pub struct CkdPrivateKey(Scalar);

impl CkdPrivateKey {
    pub fn ephemeral(rng: impl CryptoRng) -> (Self, CkdPublicKey) {
        Self::from_scalar(Scalar::random(rng.v0_10()))
    }

    fn from_scalar(x: Scalar) -> (Self, CkdPublicKey) {
        let pk1 = G1Projective::generator() * x;
        let pk2 = G2Projective::generator() * x;

        let pk = CkdPublicKey {
            pk1: pk1.to_affine(),
            pk2: pk2.to_affine(),
        };

        debug_assert!(pk.check());

        (Self(x), pk)
    }

    /// Decrypt the secret and verify that it was derived from given MPC
    /// public key for given predecessor and path.
    ///
    /// **NOTE**: Returned secret's entropy is not distrubuted uniformly, so
    /// it shouldn't be used as-is. Instead, use HKDF to derive a strong
    /// uniformly-distributed key.
    pub fn decrypt_verify(
        &self,
        mpc_public_key: G2Affine,
        predecessor_id: impl AsRef<AccountIdRef>,
        path: impl AsRef<str>,
        resp: CkdResponse,
    ) -> Option<Secret> {
        let app_id = defuse_kdf_mpc::ckd(predecessor_id).derive_path(path.as_ref());

        self.decrypt_verify_app_id(mpc_public_key, &app_id, resp)
    }

    /// Decrypt the secret and verify that it was derived from given MPC
    /// public key for given predecessor and path.
    ///
    /// **NOTE**: Returned secret's entropy is not distrubuted uniformly, so
    /// it shouldn't be used as-is. Instead, use HKDF to derive a strong
    /// uniformly-distributed key.
    pub fn decrypt_verify_app_id(
        &self,
        mpc_public_key: G2Affine,
        app_id: &[u8; 32],
        resp: CkdResponse,
    ) -> Option<Secret> {
        let secret = self.decrypt(resp).to_affine();

        if !Self::verify(mpc_public_key, app_id, secret) {
            return None;
        }

        Some(secret.to_compressed())
    }

    /// See <https://github.com/near/mpc/blob/f7a959d2bfd723e92c3bd71a5b60e03d972a2ddb/crates/ckd-example-cli/src/ckd.rs#L128-L129>
    fn decrypt(&self, resp: CkdResponse) -> G1Projective {
        resp.big_c - resp.big_y * self.0
    }

    /// Check that `e(sig, g2) = e(hash_point, mpc_public_key)`
    ///
    /// See <https://github.com/near/mpc/blob/f7a959d2bfd723e92c3bd71a5b60e03d972a2ddb/crates/ckd-example-cli/src/ckd.rs#L100-L115>
    fn verify(mpc_public_key: G2Affine, app_id: &[u8; 32], signature: G1Affine) -> bool {
        if !check_g1(&signature) || !check_g2(&mpc_public_key) {
            return false;
        }

        let hp = hash_point(&mpc_public_key, app_id);
        let minus_g2 = -G2Affine::generator();

        Bls12::multi_miller_loop(&[
            (&signature, &minus_g2.into()),
            (&hp.into(), &mpc_public_key.into()),
        ])
        .final_exponentiation()
        .is_identity()
        .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CkdPublicKey {
    pub pk1: G1Affine,
    pub pk2: G2Affine,
}

impl CkdPublicKey {
    /// Check that `e(app_pk1, g2) = e(g1, app_pk2)`
    ///
    /// See <https://github.com/near/mpc/blob/f7a959d2bfd723e92c3bd71a5b60e03d972a2ddb/crates/contract/src/primitives/ckd.rs#L34-L54>
    pub fn check(&self) -> bool {
        if !check_g1(&self.pk1) || !check_g2(&self.pk2) {
            return false;
        }

        let g1 = G1Affine::generator();
        let minus_g2 = -G2Affine::generator();

        // TODO: cfg(near)
        Bls12::multi_miller_loop(&[(&self.pk1, &minus_g2.into()), (&g1, &self.pk2.into())])
            .final_exponentiation()
            .is_identity()
            .into()
    }

    pub fn verify(
        &self,
        mpc_public_key: G2Affine,
        predecessor_id: impl AsRef<AccountIdRef>,
        path: impl AsRef<str>,
        resp: &CkdResponse,
    ) -> bool {
        let app_id = defuse_kdf_mpc::ckd(predecessor_id).derive_path(path.as_ref());

        self.verify_app_id(mpc_public_key, app_id, resp)
    }

    /// Check that `e(big_c, g2) = e(big_y, app_pk2) * e(hash_point, public_key)`
    ///
    /// See <https://github.com/near/mpc/blob/f7a959d2bfd723e92c3bd71a5b60e03d972a2ddb/crates/contract/src/primitives/ckd.rs#L56-L83>
    pub fn verify_app_id(
        &self,
        mpc_public_key: G2Affine,
        app_id: [u8; 32],
        resp: &CkdResponse,
    ) -> bool {
        let minus_g2 = -G2Affine::generator();
        let hp = hash_point(&mpc_public_key, &app_id);

        // TODO: cfg(near)
        Bls12::multi_miller_loop(&[
            (&resp.big_c, &minus_g2.into()),
            (&resp.big_y, &self.pk2.into()),
            (&hp.into(), &mpc_public_key.into()),
        ])
        .final_exponentiation()
        .is_identity()
        .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CkdResponse {
    pub big_y: G1Affine,
    pub big_c: G1Affine,
}

fn check_g1(p: &G1Affine) -> bool {
    (!p.is_identity() & p.is_on_curve() & p.is_torsion_free()).into()
}

fn check_g2(p: &G2Affine) -> bool {
    (!p.is_identity() & p.is_on_curve() & p.is_torsion_free()).into()
}

/// See <https://github.com/near/mpc/blob/f7a959d2bfd723e92c3bd71a5b60e03d972a2ddb/crates/contract/src/primitives/ckd.rs#L85-L90>
fn hash_point(mpc_public_key: &G2Affine, app_id: &[u8; 32]) -> G1Projective {
    const NEAR_CKD_DOMAIN: &[u8] = b"NEAR BLS12381G1_XMD:SHA-256_SSWU_RO_";

    G1Projective::hash_to_curve(
        &[mpc_public_key.to_compressed().as_slice(), app_id.as_slice()].concat(),
        NEAR_CKD_DOMAIN,
        b"",
    )
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    // TODO: failing cases

    // v1.signer::public_key({"domain_id": 2}) ->
    // bls12381g2:24mhN4RnB2CbiUkAfukyh4s1CT6dUNd9Pc8kRnL4LvAP3tcxhUupbphmfbmwHSi66aFCiZkMgiH2KqXWJLD7JUeAFhoLS3WQbzWcpUzhERLqxyocwT9Xrd4WNvEuKavxmXdR
    const MPC_PUBLIC_KEY: [u8; G2Affine::compressed_size()] = hex!(
        "b35176b6b2c088c534c54d253499a623217fc92589562ad0c3ef2886c5adb9674b6fa71bfd1ff9556ebb98cb4cb56d8a188e69307db1f43794cca8aa8f5ac7b2dd847d15707ffbb2e478fb92e189b36899769a654313e0dcd37f282e82cc1728"
    );

    #[rstest]
    // https://explorer.near.org/transactions/8TnEwxAkV3BfW1LJiyvujntWqDEezr763bBQADRMrJfu
    #[case(
        hex!("9862ee5a15bc4981a798344765af85670b2e6d531c9ef4eaeb233205aa0ebdb297472d1b250e03f7d560be1426bf7355"),
        hex!("8019c41510c0050966a0a180e4fc9bfd0966ad5849c08fc1329993496c3781d7d2d6ca2a3b03b1d1b16ececfe09ab7e41404a03d8066fdcd17043c06b75d203a86aa3b0c96841737f1b8a83a1973ebf26f85b5984e8f52b1f0d0bb52e93c470c"),
        "iamgrut.near",
        "mykey",
        hex!("b69c210332f7690135af86ab9283e8bcd2d67314d397b78b2512711543d492aff37d5bbf75dc5a9c82e67f3e65ef890f"),
        hex!("a3d18fca317a48fdf462245d4485e2f19a157935773295851ddb0136e1753adc36c7a978e70b65802f8472ee362b3ed5"),
    )]
    // https://explorer.near.org/transactions/2MuFgicsJ9DUMbBSDiRBbjaTk3HHHrtuCochHybutqZE
    #[case(
        hex!("a0d04c5e6507cc06b1aa0e18b39b0a3e18360cbb32825e32daf86e1bddd672b370b4cc064378364a60a063f87d6722cd"),
        hex!("ad0f8b6395fa919fbbbc8eab1991dc222950eb290cc47f6a441caf74e1fef7806588205d50cbd5d40b137edf270621951849f78534aaf7654a47552e6aaac82ba8de4bc751a5990fbca603a115335e6cad5ad408a68b56c243e98c33a75fbd9b"),
        "iamgrut.near",
        "pv1",
        hex!("8b25d70e532f4bb06c175cee35a1007359fd0eefb0aea5aebcb708f84f5a69154bb363a34eb5cfa495abe3d25802725d"),
        hex!("b5f1ebc63ace3e69948d90df673e48b520d3b3dce908fa66dccc87fe094f49608454c4691687f10f321232a32bc9d5fb"),
    )]
    fn pv_verify(
        #[case] pk1: [u8; G1Affine::compressed_size()],
        #[case] pk2: [u8; G2Affine::compressed_size()],
        #[case] predecessor_id: &str,
        #[case] path: &str,
        #[case] big_y: [u8; G1Affine::compressed_size()],
        #[case] big_c: [u8; G1Affine::compressed_size()],
    ) {
        // 1. App prepares PV (publicly-verifiable) app public key
        let pk = CkdPublicKey {
            pk1: G1Affine::from_compressed(&pk1).unwrap(),
            pk2: G2Affine::from_compressed(&pk2).unwrap(),
        };

        // 2. MPC contract checks PV app public key and emits an event
        // for MPC nodes
        assert!(pk.check());

        // 3. MPC nodes encrypt derived secret and publish the response
        // back on-chain
        let mpc_public_key = G2Affine::from_compressed(&MPC_PUBLIC_KEY).unwrap();
        let predecessor_id = AccountIdRef::new_or_panic(predecessor_id);

        let resp = CkdResponse {
            big_y: G1Affine::from_compressed(&big_y).unwrap(),
            big_c: G1Affine::from_compressed(&big_c).unwrap(),
        };

        // 4. MPC contract verifies returned signature for PV app_pk
        assert!(pk.verify(mpc_public_key, predecessor_id, path, &resp));
    }

    #[rstest]
    // https://nearblocks.io/txns/DmRZQx9Z3BT8LRV4QPtEN63DDJ7zFwZEZdPQhD3oufkT
    #[case(
        hex!("bcdfe70513e78a088045d0a308d5c4d5dc82dd2a8e6eb139f1bd31f84c4f6462"),
        "iamgrut.near",
        "test",
        hex!("b573e889bae185df627320fddde88192d590d0a30b55e0db22c972e3542e9fbe7d60e014fa381921eaf2ca45cba85692"),
        hex!("87b43dfbb9b47cd6b8e79fdf408f295775f794b6bc1ed23f866c9c5246e67c2afd2ba27e7988295d55b9b41787f85901"),
        hex!("b6b17e8a6cbbdc22be3690f93baf1f92a13cf395979c3a4d9304322a02e1aa6e2c9cc1a394fbe2adec4f23686837d67f"),
    )]
    // https://explorer.near.org/transactions/5ZMVWWdvCa458oHspq9Fh6j9iDCsUdxeohKuGCwdu4cb
    #[case(
        hex!("164de7cb437bcbc14d5fd64f8980323d0b7c8ab1de88ac0dbb2ff705ec98c317"),
        "iamgrut.near",
        "test1",
        hex!("b8d9fe539e8c65fb0d9f58ee5d564977ce2b1470750161276cd8eaaa21f8b83ac55e9f241718da5e801ba169a8aeeca6"),
        hex!("b817673a98b64f81e0e80d736ec61fad963a1300a38a48f5971e43b139571a29f662dd632cfd2e703034e5d8f1e10106"),
        hex!("abd63451d52c71f4422a2f2d1f898f60c0afaa18c74aa4b8fe355a7735982006fbbc4a9e064f37d026bb235f8ea1ace7"),
    )]
    fn decrypt_verify(
        #[case] scalar_le: [u8; 32],
        #[case] predecessor_id: &str,
        #[case] path: &str,
        #[case] big_y: [u8; G1Affine::compressed_size()],
        #[case] big_c: [u8; G1Affine::compressed_size()],
        #[case] secret: [u8; 48],
    ) {
        let scalar = Scalar::from_bytes_le(&scalar_le).unwrap();

        // 1. Application creates a keypair
        let (sk, pk) = CkdPrivateKey::from_scalar(scalar);

        // 2. MPC contract checks PV (publicly-verifiable) app public key
        // and emits an event for MPC nodes
        assert!(pk.check());

        // 3. MPC nodes encrypt derived secret and publish the response
        // back on-chain
        let mpc_public_key = G2Affine::from_compressed(&MPC_PUBLIC_KEY).unwrap();
        let predecessor_id = AccountIdRef::new_or_panic(predecessor_id);

        let resp = CkdResponse {
            big_y: G1Affine::from_compressed(&big_y).unwrap(),
            big_c: G1Affine::from_compressed(&big_c).unwrap(),
        };

        // 4. MPC contract verifies returned signature for PV app_pk
        assert!(pk.verify(mpc_public_key, predecessor_id, path, &resp));

        // 5. App decrypts the response and verifies that it was derived correctly
        assert_eq!(
            sk.decrypt_verify(mpc_public_key, predecessor_id, path, resp),
            Some(secret)
        );
    }
}
