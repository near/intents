use defuse_outlayer_host::crypto::secp256k1::{
    Secp256k1Host, Secp256k1PublicKey, Secp256k1Signature,
};
use k256::{
    NonZeroScalar, PublicKey, Scalar, SecretKey,
    ecdsa::{RecoveryId, SigningKey},
    elliptic_curve::{CurveArithmetic, ops::Reduce, sec1::ToEncodedPoint},
};

use crate::WorkerHost;

impl WorkerHost {
    /// Derives the additive tweak for a given `path`, reduced modulo
    /// the curve order so that any 32-byte digest yields a valid
    /// scalar (rejection-free; bias is negligible since `n ≈ 2^256`).
    fn secp256k1_tweak(&self, path: &str) -> Scalar {
        <Scalar as Reduce<k256::U256>>::reduce_bytes(&self.derive_tweak(path).into())
    }
}

impl Secp256k1Host for WorkerHost {
    /// Non-hardened child public-key derivation:
    ///
    /// ```text
    ///     pk(path) = root_pk + H(app_id, path) * G
    /// ```
    ///
    /// The formula depends only on `root_pk` — not on `root_sk` or
    /// the seed — so any client that knows `root_pk` can derive all
    /// child public keys fully offline. The host MUST mirror exactly
    /// this computation here so that the on-host result is bit-for-bit
    /// identical to what a client would compute.
    fn secp256k1_derive_public_key(&self, path: &str) -> Secp256k1PublicKey {
        type ProjectivePoint = <k256::Secp256k1 as CurveArithmetic>::ProjectivePoint;

        let tweak = self.secp256k1_tweak(path);

        // `root_pk + tweak * G`, computed purely from the root public key.
        let root_pk_point: ProjectivePoint = self.secp256k1_root_sk.public_key().to_projective();
        let derived_point = root_pk_point + ProjectivePoint::GENERATOR * tweak;

        // With a random `tweak`, `derived_point == 0` iff `tweak == -root_sk`,
        // which happens with probability ≈ 2^-256 — treat as unreachable.
        // `PublicKey::from_affine` rejects the identity point for us.
        let pk = PublicKey::from_affine(derived_point.to_affine())
            .expect("derived public key is the point at infinity");

        // Serialize as uncompressed SEC1 (`0x04 || X (32) || Y (32)`) and
        // strip the `0x04` tag, matching `Secp256k1PublicKey = [u8; 64]`
        // and `defuse_crypto::Secp256k1::PublicKey` (which feeds directly
        // into `near_sdk::env::ecrecover`).
        pk.to_encoded_point(false).as_bytes()[1..]
            .try_into()
            .expect("uncompressed SEC1 point is 65 bytes")
    }

    fn secp256k1_sign(&self, path: &str, prehash: &[u8; 32]) -> Secp256k1Signature {
        // Signing is the only operation that needs `root_sk`. The derived
        // secret key is
        //
        //     sk(path) = root_sk + H(app_id, path)  (mod n)
        //
        // whose public key equals the one computed in
        // `secp256k1_derive_public_key` by the linearity of scalar mult.
        let tweak = self.secp256k1_tweak(path);
        let root_scalar: Scalar = *self.secp256k1_root_sk.to_nonzero_scalar();
        let derived_scalar = NonZeroScalar::new(root_scalar + tweak)
            .into_option()
            .expect("derived secret key is zero");
        let derived_sk = SecretKey::from(derived_scalar);
        let signing_key = SigningKey::from(&derived_sk);

        // `sign_prehash_recoverable` applies low-S normalization, so the
        // resulting signature is non-malleable and accepted by
        // `env::ecrecover(.., malleability_flag = true)`.
        let (signature, recovery_id): (k256::ecdsa::Signature, RecoveryId) = signing_key
            .sign_prehash_recoverable(prehash)
            .expect("prehash signing is infallible for a 32-byte input");

        let mut out = [0u8; 65];
        out[..64].copy_from_slice(&signature.to_bytes());
        out[64] = recovery_id.to_byte();
        out
    }
}

#[cfg(test)]
mod tests {
    use k256::{
        ecdsa::{Signature, VerifyingKey},
        elliptic_curve::sec1::ToEncodedPoint,
    };
    use near_account_id::AccountId;

    use crate::{AppId, WorkerHost};

    use super::*;

    fn host() -> WorkerHost {
        let seed = [42u8; 64];
        let app_id = AppId::NearAccount("app.near".parse::<AccountId>().unwrap());
        WorkerHost::new(app_id, &seed)
    }

    /// The host's public-key derivation must match the "offline" formula
    /// `pk(path) = root_pk + tweak * G` that a client would use with only
    /// the root public key — demonstrating that no access to `root_sk` or
    /// the seed is required to reproduce the result.
    #[test]
    fn derive_public_key_matches_offline_formula() {
        type ProjectivePoint = <k256::Secp256k1 as CurveArithmetic>::ProjectivePoint;

        let host = host();
        let root_pk = host.secp256k1_root_sk.public_key();

        for path in ["", "a", "a/b", "deeply/nested/path"] {
            let tweak = host.secp256k1_tweak(path);

            // Client-side computation, using only `root_pk`.
            let expected_point = root_pk.to_projective() + ProjectivePoint::GENERATOR * tweak;
            let expected = expected_point.to_affine().to_encoded_point(false);
            let expected: [u8; 64] = expected.as_bytes()[1..].try_into().unwrap();

            assert_eq!(host.secp256k1_derive_public_key(path), expected);
        }
    }

    /// The derived public key returned by `secp256k1_derive_public_key`
    /// must equal the public key of the secret key that
    /// `secp256k1_sign` uses internally — i.e. signatures verify against
    /// what clients derive offline.
    #[test]
    fn derived_public_key_matches_signing_key() {
        let host = host();

        for path in ["", "x", "some/path"] {
            let derived_pk = host.secp256k1_derive_public_key(path);

            // Reconstruct the signing key the same way `secp256k1_sign` does.
            let tweak = host.secp256k1_tweak(path);
            let sk_scalar: Scalar = *host.secp256k1_root_sk.to_nonzero_scalar();
            let nz = NonZeroScalar::new(sk_scalar + tweak).unwrap();
            let sk_pk = SecretKey::from(nz).public_key().to_encoded_point(false);
            let sk_pk: [u8; 64] = sk_pk.as_bytes()[1..].try_into().unwrap();

            assert_eq!(derived_pk, sk_pk);
        }
    }

    /// End-to-end guarantee the verifying side relies on: running
    /// ECDSA public-key recovery on `(msg, signature)` produced by
    /// `secp256k1_sign(path, msg)` must yield the same public key as
    /// `secp256k1_derive_public_key(path)`.
    ///
    /// This is the exact operation `near_sdk::env::ecrecover` performs
    /// on-chain (with `malleability_flag = true`); k256's
    /// `VerifyingKey::recover_from_prehash` implements the same
    /// standardized recovery algorithm, and `sign_prehash_recoverable`
    /// already normalizes `s` to its low form, so a recovered key
    /// from our signature will also be accepted by the on-chain path.
    #[test]
    fn ecrecover_roundtrip_matches_derive_public_key() {
        let host = host();

        let messages: [[u8; 32]; 3] = [[0u8; 32], [0xAB; 32], core::array::from_fn(|i| i as u8)];

        for path in ["", "a", "wallet/0", "deeply/nested/path"] {
            let expected_pk = host.secp256k1_derive_public_key(path);

            for msg in &messages {
                let sig_bytes = host.secp256k1_sign(path, msg);

                // Unpack `r || s || v` as emitted by `secp256k1_sign`;
                // this mirrors `defuse_crypto::Secp256k1::verify`.
                let [rs @ .., v] = sig_bytes;
                let signature = Signature::from_slice(&rs).expect("valid (r, s)");
                let recovery_id = RecoveryId::from_byte(v).expect("v ∈ {0, 1}");

                // Sanity-check that `sign_prehash_recoverable` produced
                // a low-S signature — otherwise `env::ecrecover` with
                // `malleability_flag = true` would reject it on-chain.
                assert!(
                    signature.normalize_s().is_none(),
                    "signature for path={path:?} is not in low-S form",
                );

                let recovered = VerifyingKey::recover_from_prehash(msg, &signature, recovery_id)
                    .expect("recovery must succeed for a signature we just produced");

                // Re-encode the recovered key in the same wire format as
                // `secp256k1_derive_public_key`: uncompressed SEC1 with the
                // `0x04` tag stripped.
                let recovered: [u8; 64] = recovered.to_encoded_point(false).as_bytes()[1..]
                    .try_into()
                    .unwrap();

                assert_eq!(
                    recovered, expected_pk,
                    "recovered pk differs from derived pk for path={path:?}",
                );
            }
        }
    }
}
