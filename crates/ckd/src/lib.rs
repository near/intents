use blstrs::{Bls12, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Scalar};
use defuse_kdf_mpc::kdf::Schema;
use pairing::{
    MillerLoopResult, MultiMillerLoop,
    group::{Curve, Group, ff::Field, prime::PrimeCurveAffine},
};

use near_account_id::AccountIdRef;
use rand_core::CryptoRng;
// use zeroize::ZeroizeOnDrop;

const NEAR_CKD_DOMAIN: &[u8] = b"NEAR BLS12381G1_XMD:SHA-256_SSWU_RO_";

// // TODO
// // #[derive(ZeroizeOnDrop)]
pub struct Ckd {
    private_key: Scalar,
}

// // TODO
// // let app_id = defuse_kdf_mpc::ckd(predecessor_id).derive_path(path);

impl Ckd {
    // // TODO: implement Distribution?
    // pub fn random(rng: impl CryptoRng) -> (Self, G1Projective, G2Projective) {
    //     // TODO: legacy adaptor
    //     Self::from_scalar(Scalar::random(todo!()))
    // }

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
    ) -> Option<[u8; G1Affine::compressed_size()]> {
        let secret = self.decrypt(big_y, big_c);

        // if !Self::verify_legacy(mpc_public_key, secret) {
        //     return None;
        // }

        Some(secret.to_compressed())
    }

    fn decrypt(&self, big_y: G1Projective, big_c: G1Projective) -> G1Projective {
        // TODO: cfg(near)
        // env::bls12381_p1_sum() + env::bls12381_g1_multiexp()
        big_c - big_y * self.private_key
    }

    // fn verify_legacy(public_key: G2Projective, signature: G1Projective) -> bool {
    //     let element1 = signature.to_affine();
    //     if (!element1.is_on_curve() | !element1.is_torsion_free() | element1.is_identity()).into() {
    //         return false;
    //     }

    //     let element2 = public_key.to_affine();
    //     if (!element2.is_on_curve() | !element2.is_torsion_free() | element2.is_identity()).into() {
    //         return false;
    //     }

    //     let hash_input = [public_key.to_compressed().as_slice(), app_id].concat();
    //     let hash_point = G1Projective::hash_to_curve(&hash_input, NEAR_CKD_DOMAIN, b"").into();
    //     let base2 = G2Affine::generator();

    //     // TODO: cfg(near)
    //     pairing(&hash_point, &element2) == pairing(&element1, &base2)
    // }

    // fn verify_pv(
    //     app_id: [u8; 32],
    //     app_pk2: G2Affine,
    //     big_y: G1Affine,
    //     big_c: G1Affine,
    //     public_key: G2Affine,
    // ) -> bool {
    //     let minus_g2 = -G2Affine::generator();

    //     let hash_point: G1Affine = G1Projective::hash_to_curve(
    //         &[public_key.to_compressed().as_slice(), app_id.as_slice()].concat(),
    //         NEAR_CKD_DOMAIN,
    //         b"",
    //     )
    //     .into();

    //     let ml = Bls12::multi_miller_loop(&[
    //         (&big_c, &minus_g2.into()),
    //         (&big_y, &app_pk2.into()),
    //         (&hash_point, &public_key.into()),
    //     ]);

    //     let res = ml.final_exponentiation();

    //     res.is_identity().into()
    // }
}

// pub struct CkdClient {
//     app_id: [u8; 32],
//     app_pk2: G2Prepared,
// }

// impl CkdClient {
//     pub fn verify(&self, mpc_public_key: G2Prepared, big_y: &G1Affine, big_c: &G1Affine) -> bool {}
// }

pub struct CkdSession {
    mpc_public_key: G2Prepared,
    hash_point: G1Affine,
    app_pk2: G2Prepared,
}

impl CkdSession {
    pub fn new(
        mpc_public_key: G2Affine,
        predecessor_id: impl AsRef<AccountIdRef>,
        path: impl AsRef<str>,
        app_pk2: G2Prepared,
    ) -> Self {
        let app_id = defuse_kdf_mpc::ckd(predecessor_id).derive_path(path.as_ref());

        Self::new_with_app_id(mpc_public_key, app_id, app_pk2)
    }

    fn new_with_app_id(mpc_public_key: G2Affine, app_id: [u8; 32], app_pk2: G2Prepared) -> Self {
        let hash_point = G1Projective::hash_to_curve(
            &[mpc_public_key.to_compressed().as_slice(), app_id.as_slice()].concat(),
            NEAR_CKD_DOMAIN,
            b"",
        );

        Self {
            mpc_public_key: mpc_public_key.into(),
            hash_point: hash_point.into(),
            app_pk2,
        }
    }

    pub fn verify(&self, big_y: &G1Affine, big_c: &G1Affine) -> bool {
        thread_local! {
            // prepare only once per thread
            static MINUS_G2: G2Prepared = (-G2Affine::generator()).into();
        }

        // TODO: cfg(near)
        MINUS_G2
            .with(|minus_g2| {
                Bls12::multi_miller_loop(&[
                    (big_c, minus_g2),
                    (big_y, &self.app_pk2),
                    (&self.hash_point, &self.mpc_public_key),
                ])
            })
            .final_exponentiation()
            .is_identity()
            .into()
    }
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    const MPC_PUBLIC_KEY: [u8; G2Affine::compressed_size()] = hex!(
        "b35176b6b2c088c534c54d253499a623217fc92589562ad0c3ef2886c5adb9674b6fa71bfd1ff9556ebb98cb4cb56d8a188e69307db1f43794cca8aa8f5ac7b2dd847d15707ffbb2e478fb92e189b36899769a654313e0dcd37f282e82cc1728"
    );

    // #[rstest]
    // #[case(
    //     hex!(""),
    // )]
    // fn test(#[case] secret: [u8; 48]) {
    //     // let (ckd, p1, p2) = Ckd::new();
    //     // let secret = ckd
    //     //     .decrypt_and_verify(big_y, big_c, mpc_public_key)
    //     //     .expect("verification failed");

    //     // assert_eq!(secret,)
    // }

    // https://explorer.near.org/transactions/8TnEwxAkV3BfW1LJiyvujntWqDEezr763bBQADRMrJfu
    // private key be: 2660caf7a7b53715c6a0c623cfde8ff7a7137315f33888a2857840dca247fea1
    // mpc public key: bls12381g2:24mhN4RnB2CbiUkAfukyh4s1CT6dUNd9Pc8kRnL4LvAP3tcxhUupbphmfbmwHSi66aFCiZkMgiH2KqXWJLD7JUeAFhoLS3WQbzWcpUzhERLqxyocwT9Xrd4WNvEuKavxmXdR

    // https://explorer.near.org/transactions/DmRZQx9Z3BT8LRV4QPtEN63DDJ7zFwZEZdPQhD3oufkT
    // private key be: 62644f4cf831bdf139b16e8e2add82dcd5c4d508a3d04580088ae71305e7dfbc
    // secret: b6b17e8a6cbbdc22be3690f93baf1f92a13cf395979c3a4d9304322a02e1aa6e2c9cc1a394fbe2adec4f23686837d67f

    #[rstest]
    // TODO: failing cases
    #[case(
        hex!("bcdfe70513e78a088045d0a308d5c4d5dc82dd2a8e6eb139f1bd31f84c4f6462"),
        "iamgrut.near",
        "test",
        hex!("87b43dfbb9b47cd6b8e79fdf408f295775f794b6bc1ed23f866c9c5246e67c2afd2ba27e7988295d55b9b41787f85901"),
        hex!("b573e889bae185df627320fddde88192d590d0a30b55e0db22c972e3542e9fbe7d60e014fa381921eaf2ca45cba85692"),
        hex!("b6b17e8a6cbbdc22be3690f93baf1f92a13cf395979c3a4d9304322a02e1aa6e2c9cc1a394fbe2adec4f23686837d67f"),
    )]
    fn verify(
        #[case] scalar_le: [u8; 32],
        #[case] predecessor_id: &str,
        #[case] path: &str,
        #[case] big_c: [u8; G1Affine::compressed_size()],
        #[case] big_y: [u8; G1Affine::compressed_size()],
        #[case] secret: [u8; 48],
    ) {
        let scalar = Scalar::from_bytes_le(&scalar_le).unwrap();

        let (ckd, _app_pk1, app_pk2) = Ckd::from_scalar(scalar);

        let mpc_public_key = G2Affine::from_compressed(&MPC_PUBLIC_KEY).unwrap();

        let app_pk2 = app_pk2.to_affine();
        // let app_pk2 = G2Affine::from_compressed(&app_pk2).unwrap();
        let big_c = G1Affine::from_compressed(&big_c).unwrap();
        let big_y = G1Affine::from_compressed(&big_y).unwrap();

        let session = CkdSession::new(
            mpc_public_key,
            AccountIdRef::new_or_panic(predecessor_id),
            path,
            app_pk2.into(),
        );

        assert!(session.verify(&big_y, &big_c));

        assert_eq!(
            ckd.decrypt_and_verify(big_y.into(), big_c.into()),
            Some(secret)
        );
    }
}

/*
cargo run -p ckd-example-cli -- --domain-id 2 --signer-account-id iamgrut.near --derivation-path "test" --mpc-ckd-public-key bls12381g2:24mhN4RnB2CbiUkAfukyh4s1CT6dUNd9Pc8kRnL4LvAP3tcxhUupbphmfbmwHSi66aFCiZkMgiH2KqXWJLD7JUeAFhoLS3WQbzWcpUzhERLqxyocwT9Xrd4WNvEuKavxmXdR --publicly-verifiable
   Compiling ckd-example-cli v3.10.0 (/Users/mitinarseny/dev/near/mpc/crates/ckd-example-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.94s
     Running `target/debug/ckd-example-cli --domain-id 2 --signer-account-id iamgrut.near --derivation-path test --mpc-ckd-public-key 'bls12381g2:24mhN4RnB2CbiUkAfukyh4s1CT6dUNd9Pc8kRnL4LvAP3tcxhUupbphmfbmwHSi66aFCiZkMgiH2KqXWJLD7JUeAFhoLS3WQbzWcpUzhERLqxyocwT9Xrd4WNvEuKavxmXdR' --publicly-verifiable`
app_private_key: 62644f4cf831bdf139b16e8e2add82dcd5c4d508a3d04580088ae71305e7dfbc
Call the function request_app_private_key with parameters:
{"request":{"app_public_key":{"AppPublicKeyPV":{"pk1":"bls12381g1:783YAkZAXnRJQbED55PD2h3w2BhD42M2i9j6g9FjD5BjCsF8ZPW7H6XLWXmFBDefK2","pk2":"bls12381g2:rZUd9apMLxHKfrcE152CovyXMnsgnW54HRraXbYVihVeB962mcmWbofXkacqkj1Aa3iwP1xJ99moVdxiHCdv37voaTzZPfmLuAuGoCQNXsa6ap1ehvwJpqNMvrLrAACptK9"}},"derivation_path":"test","domain_id":2}}
Please enter a the response in json format (for example {"big_c": "bls12381g1:...","big_y": "bls12381g1:..."}):
Your response: {
  "big_c": "bls12381g1:5yisKJW5bwodkkJ2YicetMC16C3KDZHjk49cmztfB4MC4Mg7XH9MaEhvQiKyr3Wtek",
  "big_y": "bls12381g1:7f4FbTjkNRthybTfFu7gVDA4ieA51f3RtPXYTKfadca1Wk5YW3aVcfWPyXTzGutV8u"
}
secret: b6b17e8a6cbbdc22be3690f93baf1f92a13cf395979c3a4d9304322a02e1aa6e2c9cc1a394fbe2adec4f23686837d67f
The key is: 82fc4d3981f1d333cf620920eb57891c533e38cc326a6b7ada361e8585a3d2fe
*/
