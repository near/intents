//! [SEP-53](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0053.md) Signed Data Standard

use defuse_digest::{Digest, sha2::Sha256};
use defuse_kdf_crypto::{Curve, Ed25519};

/// Verify signature over a given message for given public key according to
/// [SEP-53](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0053.md).
#[must_use = "check if verification passed"]
#[inline]
pub fn verify(
    public_key: &<Ed25519 as Curve>::PublicKey,
    msg: impl AsRef<[u8]>,
    signature: &<Ed25519 as Curve>::Signature,
) -> bool {
    Ed25519::verify(public_key, &prehash(msg.as_ref()), signature)
}

/// Derive prehash for signing according to following schema:
///
/// ```text
/// <"Stellar Signed Message:\n"> <data to sign>
/// ```
#[inline]
fn prehash(msg: &[u8]) -> [u8; 32] {
    Sha256::new_with_prefix(b"Stellar Signed Message:\n")
        // <data to sign>
        .chain_update(msg)
        .finalize()
        .into()
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
