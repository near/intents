use defuse_digest::{Digest, sha2::Sha256};
use defuse_signature_schema::{Result, Schema};

use crate::Algorithm;

/// [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// P256 (a.k.a secp256r1) over SHA-256
#[derive(Debug, Clone, Copy, Default)]
pub struct P256;

impl<M> Schema<M> for P256
where
    M: AsRef<[u8]>,
{
    type Output = [u8; 32];

    fn derive(&self, input: M) -> Result<Self::Output> {
        Ok(Sha256::digest(input).into())
    }
}

impl Algorithm for P256 {
    type Curve = defuse_kdf_crypto::p256::P256;
}

#[cfg(test)]
mod tests {
    use defuse_kdf_crypto::p256::{
        EncodedPoint,
        ecdsa::{Signature, VerifyingKey},
    };
    use hex_literal::hex;
    use rstest::rstest;

    use defuse_signature_schema::SignatureSchema;
    use serde_json::json;

    use crate::{UserVerification, Webauthn, WebauthnPayload};

    use super::*;

    #[rstest]
    fn verify_ok() {
        let public_key = VerifyingKey::from_encoded_point(
            &EncodedPoint::from_untagged_bytes(&hex!(
            "4a45d6946bfd801476fc90137b01d6b3c9acc7d303223264a5f5562cce7f69a1df5683656e7c56317204f3216b03536f269b2ff92a33f5bc04482ed5862aac3b"
        ).into())).unwrap();
        let payload: WebauthnPayload = serde_json::from_value(json!({
            "client_data_json": r#"{"type":"webauthn.get","challenge":"6ULo-LNIjd8Gh1mdxzUdHzv2AuGDWMchOORdDnaLXHc","origin":"https://defuse-widget-git-feat-passkeys-defuse-94bbc1b2.vercel.app"}"#,

            "authenticator_data": "933cQogpBzE3RSAYSAkfWoNEcBd3X84PxE8iRrRVxMgdAAAAAA=="
        })).unwrap();
        let signature = Signature::from_bytes(&hex!("e460a39dcb12d91fd417cab1ed26043806ea391e3129bf85e1b547fd6682fe163f4154c99a8c1ffd9f7ca470dbe14a574e1d477dda451dc004dda7a08cf9ef3f").into()).unwrap();

        assert!(payload.check(
            hex!("e942e8f8b3488ddf0687599dc7351d1f3bf602e18358c72138e45d0e768b5c77"),
            UserVerification::Require
        ));

        assert!(
            Webauthn::<P256>::default()
                .verify(&public_key, payload, &signature)
                .unwrap()
        );
    }
}
