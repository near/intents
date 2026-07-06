use defuse_digest::{Digest, sha2::Sha256};
use defuse_kdf_crypto::Ed25519;
use defuse_signature_schema::{Result, Schema, SignatureSchema};

/// [SEP-53](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0053.md)
#[derive(Debug, Clone, Copy, Default)]
pub struct Sep53;

impl<M> Schema<M> for Sep53
where
    M: AsRef<[u8]>,
{
    type Output = [u8; 32];

    /// Derive hash for signing
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hex_literal::hex;
    /// use defuse_sep53::Sep53;
    /// use defuse_signature_schema::Schema;
    ///
    /// assert_eq!(
    ///     Sep53.derive("Hello, World!").unwrap(),
    ///     hex!("aa05af77f274774b8bdc7b61d98bc40da523dc2821fdea555f4d6aa413199bcc"),
    /// );
    /// ```
    fn derive(&self, msg: M) -> Result<Self::Output> {
        thread_local! {
            // per-thread lazily-initialized hasher with pre-processed prefix
            static HASHER: Sha256 = Sha256::new_with_prefix(b"Stellar Signed Message:\n");
        }

        Ok(HASHER
            .with(Clone::clone)
            .chain_update(msg)
            .finalize()
            .into())
    }
}

impl<M> SignatureSchema<M> for Sep53
where
    M: AsRef<[u8]>,
{
    type Curve = Ed25519;
}

// TODO
// #[cfg(test)]
// mod tests {
//     use defuse_kdf_crypto::ed25519_dalek::{PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
//     use hex_literal::hex;
//     use rstest::rstest;

//     use super::*;

//     #[rstest]
//     #[case(
//         hex!("SAKICEVQLYWGSOJS4WW7HZJWAHZVEEBS527LHK5V4MLJALYKICQCJXMW"),
//         "Hello world!",
//         hex!("7cee5d6d885752104c85eea421dfdcb95abf01f1271d11c4bec3fcbd7874dccd6e2e98b97b8eb23b643cac4073bb77de5d07b0710139180ae9f3cbba78f2ba04"),
//     )]
//     fn verify_ok(
//         #[case] public_key: [u8; PUBLIC_KEY_LENGTH],
//         #[case] msg: impl AsRef<[u8]>,
//         #[case] signature: [u8; SIGNATURE_LENGTH],
//     ) {
//         let msg = msg.as_ref();
//         let [signature @ .., v] = signature;
//         let public_key = VerifyingKey::from_encoded_point(&EncodedPoint::from_untagged_bytes(
//             &public_key.into(),
//         ))
//         .unwrap();
//         let signature = Signature::from_bytes(&signature.into()).unwrap();
//         let recovery_id = RecoveryId::from_byte(v).unwrap();

//         assert!(Sep53.verify(&public_key, msg, &signature).unwrap());

//         assert_eq!(
//             Sep53.recover(msg, &signature, recovery_id).unwrap(),
//             Some(public_key)
//         );
//     }
// }
