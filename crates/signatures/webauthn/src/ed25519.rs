use crate::Algorithm;

/// [COSE EdDSA (-8) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// ed25519 curve
pub struct Ed25519;

impl Algorithm for Ed25519 {
    type Curve = defuse_crypto::ed25519::Ed25519;

    fn preprocess(msg: impl AsRef<[u8]>) -> impl AsRef<[u8]> {
        // ed25519 does the hashing inside
        msg
    }
}

#[cfg(test)]
mod tests {
    use defuse_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
    use hex_literal::hex;
    use rstest::rstest;

    use crate::{RequireUserVerification, Webauthn, WebauthnAssertion};

    use super::*;

    #[rstest]
    #[case(
        hex!("e2e9cb7ac57cb46d4da1ce1d1cc2c33bdfe17407c517916b522724a8ea2c6c50"),
        hex!("06f269191431372337a0c606a15822e349bd0d5ec317704f97bef1a4ed6f5e1d"),
        WebauthnAssertion {
            authenticator_data: hex!("49960de5880e8c687434170f6476605b8fe4aeb9a28632c7995cf3ba831d97631d00000000").to_vec(),
            client_data_json: r#"{"type":"webauthn.get","challenge":"BvJpGRQxNyM3oMYGoVgi40m9DV7DF3BPl77xpO1vXh0","origin":"http://localhost:5173","crossOrigin":false}"#.to_string(),
        },
        hex!("7cd68c54af557c3d5d7bb6810d90a3efd0eb09e11d13feae3df589d0a54e5629c56dd4e4f6ce48766fccd305135edcbfa1928b0e3131930825c464a68c7d6d0b"),
    )]
    fn verify_ok(
        #[case] public_key: impl Into<Ed25519PublicKey>,
        #[case] challenge: impl AsRef<[u8]>,
        #[case] payload: WebauthnAssertion,
        #[case] signature: impl Into<Ed25519Signature>,
    ) {
        assert!(
            Webauthn::<Ed25519, RequireUserVerification>::verify(
                &public_key.into().try_into().unwrap(),
                challenge,
                &payload,
                &signature.into().into(),
            ),
            "signature is invalid",
        );
    }
}
