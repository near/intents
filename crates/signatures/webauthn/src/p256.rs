use defuse_digest::{Digest, sha2::Sha256};

use crate::Algorithm;

/// [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// P256 (a.k.a secp256r1) over SHA-256
pub struct P256;

impl Algorithm for P256 {
    type Curve = defuse_crypto::p256::P256;

    fn preprocess(msg: impl AsRef<[u8]>) -> impl AsRef<[u8]> {
        // prehash via SHA-256
        Sha256::digest(msg)
    }
}

#[cfg(test)]
mod tests {
    use defuse_crypto::p256::{P256CompressedPublicKey, P256Signature};
    use hex_literal::hex;
    use rstest::rstest;

    use crate::{RequireUserVerification, Webauthn, WebauthnAssertion};

    use super::*;

    #[rstest]
    #[case(
        hex!("03b87da10683d04e6ec4e2f1775556a63cbb01be843058eb737fe02f9e22663093"),
        hex!("06f269191431372337a0c606a15822e349bd0d5ec317704f97bef1a4ed6f5e1d"),
        WebauthnAssertion {
            authenticator_data: hex!("49960de5880e8c687434170f6476605b8fe4aeb9a28632c7995cf3ba831d97631d00000000").to_vec(),
            client_data_json: r#"{"type":"webauthn.get","challenge":"BvJpGRQxNyM3oMYGoVgi40m9DV7DF3BPl77xpO1vXh0","origin":"http://localhost:5173","crossOrigin":false}"#.to_string(),
        },
        hex!("002527c17a17d709ab62ffab033e0a26d901f5bee6686a0d605032828e490a7c47a9f4161e6a17688ae42a66adb67c9c22c86153afb08103b1e9eccd5314c271"),
    )]
    fn verify_ok(
        #[case] public_key: impl Into<P256CompressedPublicKey>,
        #[case] challenge: impl AsRef<[u8]>,
        #[case] payload: WebauthnAssertion,
        #[case] signature: impl Into<P256Signature>,
    ) {
        assert!(
            Webauthn::<P256, RequireUserVerification>::verify(
                &public_key.into().try_into().unwrap(),
                challenge,
                &payload,
                &signature.into().try_into().unwrap(),
            ),
            "signature is invalid",
        );
    }
}
