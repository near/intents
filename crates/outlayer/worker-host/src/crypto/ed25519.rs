use curve25519_dalek::{EdwardsPoint, Scalar};
use defuse_outlayer_host::crypto::ed25519::{Ed25519Host, Ed25519PublicKey, Ed25519Signature};
use ed25519_dalek::{
    VerifyingKey,
    hazmat::{ExpandedSecretKey, raw_sign},
};
use sha2::Sha512;

use crate::WorkerHost;

impl WorkerHost {
    /// Derives the additive tweak for a given `path`, reduced modulo
    /// the Ed25519 group order `L` so that any 32-byte digest yields
    /// a valid scalar in `[0, L)`.
    ///
    /// `L ≈ 2^252`, so a 256-bit digest reduced mod `L` has a ~2^-4
    /// bias, which is cryptographically irrelevant for a tweak that
    /// is added to a uniformly random root scalar — the resulting
    /// `sk(path) = root_scalar + tweak (mod L)` remains uniform.
    fn ed25519_tweak(&self, path: &str) -> Scalar {
        Scalar::from_bytes_mod_order(self.derive_tweak(path))
    }
}

impl Ed25519Host for WorkerHost {
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
    fn ed25519_derive_public_key(&self, path: &str) -> Ed25519PublicKey {
        let tweak = self.ed25519_tweak(path);

        // `root_pk + tweak * G`, computed purely from the root public key.
        let root_pk_point = EdwardsPoint::mul_base(&self.ed25519_root_scalar);
        let derived_point = root_pk_point + EdwardsPoint::mul_base(&tweak);

        // With a random `tweak`, `derived_point == 0` iff
        // `tweak == -root_scalar (mod L)`, which happens with
        // probability ≈ 2^-252 — treat as unreachable. A zero
        // point would compress to a low-order (weak) public key
        // that downstream `Ed25519::verify` rejects anyway.
        //
        // Serialize as a 32-byte compressed Edwards Y point, matching
        // `Ed25519PublicKey = [u8; 32]` and `defuse_crypto::Ed25519`
        // (which feeds directly into `near_sdk::env::ed25519_verify`).
        VerifyingKey::from(derived_point).to_bytes()
    }

    fn ed25519_sign(&self, path: &str, msg: &[u8]) -> Ed25519Signature {
        // Signing is the only operation that needs `root_scalar`.
        // The derived secret scalar is
        //
        //     sk(path) = root_scalar + H(app_id, path)  (mod L)
        //
        // whose public key equals the one computed in
        // `ed25519_derive_public_key` by the linearity of scalar mult.
        let tweak = self.ed25519_tweak(path);
        let derived_scalar = self.ed25519_root_scalar + tweak;

        // `ExpandedSecretKey` is the hazmat-level Ed25519 signing key:
        // `(scalar, hash_prefix)` used directly by `raw_sign` without
        // the RFC 8032 seed-hash/clamp step, so we can feed in our
        // additively-derived scalar verbatim. The shared `hash_prefix`
        // is safe across paths: even if two paths sign the same `msg`
        // with the same nonce `R = H(prefix || msg) * G`, the Ed25519
        // challenge `c = H(R || A || msg)` binds to each path's
        // distinct public key `A`, so no scalar can be recovered.
        let esk = ExpandedSecretKey {
            scalar: derived_scalar,
            hash_prefix: self.ed25519_root_prefix,
        };
        // `VerifyingKey::from(&esk)` computes `derived_scalar * G`,
        // matching `ed25519_derive_public_key(path)` by construction.
        let verifying_key = VerifyingKey::from(&esk);

        // `raw_sign::<Sha512>` is the standard RFC 8032 Ed25519
        // signing algorithm (EdDSA, not Ed25519ph), so the resulting
        // signature verifies under every standard Ed25519 verifier,
        // including `near_sdk::env::ed25519_verify`.
        raw_sign::<Sha512>(&esk, msg, &verifying_key).to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use curve25519_dalek::edwards::CompressedEdwardsY;
    use ed25519_dalek::Signature;
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
    /// the root public key — demonstrating that no access to `root_scalar`
    /// or the seed is required to reproduce the result.
    #[test]
    fn derive_public_key_matches_offline_formula() {
        let host = host();
        let root_pk_point = EdwardsPoint::mul_base(&host.ed25519_root_scalar);

        for path in ["", "a", "a/b", "deeply/nested/path"] {
            let tweak = host.ed25519_tweak(path);

            // Client-side computation, using only `root_pk`.
            let expected_point = root_pk_point + EdwardsPoint::mul_base(&tweak);
            let expected: [u8; 32] = expected_point.compress().to_bytes();

            assert_eq!(host.ed25519_derive_public_key(path), expected);
        }
    }

    /// The derived public key returned by `ed25519_derive_public_key`
    /// must equal the public key of the scalar that `ed25519_sign`
    /// uses internally — i.e. signatures verify against what clients
    /// derive offline.
    #[test]
    fn derived_public_key_matches_signing_key() {
        let host = host();

        for path in ["", "x", "some/path"] {
            let derived_pk = host.ed25519_derive_public_key(path);

            // Reconstruct the signing key the same way `ed25519_sign` does.
            let tweak = host.ed25519_tweak(path);
            let sk_scalar = host.ed25519_root_scalar + tweak;
            let sk_pk: [u8; 32] = EdwardsPoint::mul_base(&sk_scalar).compress().to_bytes();

            assert_eq!(derived_pk, sk_pk);
        }
    }

    /// End-to-end guarantee the verifying side relies on: running the
    /// standard Ed25519 verification on `(msg, signature)` produced by
    /// `ed25519_sign(path, msg)` must succeed against the public key
    /// returned by `ed25519_derive_public_key(path)`.
    ///
    /// This mirrors exactly what `near_sdk::env::ed25519_verify`
    /// performs on-chain (RFC 8032 `EdDSA`, without the strict-R checks
    /// of `verify_strict`), so a signature we produce here will also
    /// be accepted by the on-chain path.
    #[test]
    fn verify_roundtrip_matches_derive_public_key() {
        let host = host();

        let messages: &[&[u8]] = &[b"", b"hello", &[0xAB; 32], &(0u8..=255).collect::<Vec<_>>()];

        for path in ["", "a", "wallet/0", "deeply/nested/path"] {
            let pk_bytes = host.ed25519_derive_public_key(path);
            let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
                .expect("derived public key is a valid compressed Edwards point");
            // The derived key must not be one of the low-order
            // ("weak") points — else `defuse_crypto::Ed25519::verify`
            // would refuse to verify any signature under it.
            assert!(
                !verifying_key.is_weak(),
                "derived public key for path={path:?} is a weak (low-order) key",
            );

            for msg in messages {
                let sig_bytes = host.ed25519_sign(path, msg);
                let signature = Signature::from_bytes(&sig_bytes);

                verifying_key.verify_strict(msg, &signature).unwrap_or_else(|e| {
                    panic!(
                        "signature for path={path:?}, msg_len={} failed to verify: {e}",
                        msg.len(),
                    );
                });
            }
        }
    }

    /// Every byte of the derived public key must be a canonical
    /// compressed Edwards point — i.e. `VerifyingKey::from_bytes`
    /// accepts it and round-trips to the same 32 bytes.
    #[test]
    fn derived_public_key_is_canonical() {
        let host = host();

        for path in ["", "a", "wallet/0", "deeply/nested/path"] {
            let pk = host.ed25519_derive_public_key(path);

            // Canonical compressed Y encoding.
            let compressed = CompressedEdwardsY(pk);
            let decoded = compressed
                .decompress()
                .expect("derived public key must decompress to a valid Edwards point");
            assert_eq!(decoded.compress().to_bytes(), pk);
        }
    }
}
