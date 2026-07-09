//! [TIP-191](https://github.com/tronprotocol/tips/blob/master/tip-191.md)
//! Signed Data Standard

use defuse_crypto::{Curve, RecoverableCurve, secp256k1::Secp256k1};
use defuse_digest::{Digest, sha3::Keccak256};

/// [TIP-191](https://github.com/tronprotocol/tips/blob/master/tip-191.md)
/// Signed Data Standard
pub struct Tip191;

impl Tip191 {
    /// Try to recover public key which signed given message according to
    /// [TIP-191](https://github.com/tronprotocol/tips/blob/master/tip-191.md)
    /// and produced given signature and recovery id.
    #[must_use = "check recovered public key"]
    #[inline]
    pub fn recover(
        msg: impl AsRef<[u8]>,
        signature: &<Secp256k1 as Curve>::Signature,
        recovery_id: <Secp256k1 as RecoverableCurve>::RecoveryId,
    ) -> Option<<Secp256k1 as Curve>::PublicKey> {
        Secp256k1::recover(&Self::prehash(msg.as_ref()), signature, recovery_id)
    }

    /// Derive prehash for signing
    #[inline]
    pub fn prehash(msg: impl AsRef<[u8]>) -> [u8; 32] {
        let msg = msg.as_ref();

        // Prefix itself is not specified in the standard. But from:
        // https://tronweb.network/docu/docs/Sign%20and%20Verify%20Message/
        Keccak256::new_with_prefix(b"\x19TRON Signed Message:\n")
            // `len(message)` is the non-zero-padded ascii-decimal encoding of the number of bytes in message.
            .chain_update(msg.len().to_string())
            // <data to sign>
            .chain_update(msg)
            .finalize()
            .into()
    }
}

#[cfg(test)]
mod tests {
    use defuse_crypto::secp256k1::k256::{
        EncodedPoint,
        ecdsa::{RecoveryId, Signature, VerifyingKey},
    };
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"),
        "Hello, TRON!",
        hex!("eea1651a60600ec4d9c45e8ae81da1a78377f789f0ac2019de66ad943459913015ef9256809ee0e6bb76e303a0b4802e475c1d26ade5d585292b80c9fe9cb10c01"),
    )]
    fn recover_ok(
        #[case] public_key: [u8; 64],
        #[case] msg: impl AsRef<[u8]>,
        #[case] signature: [u8; 65],
    ) {
        let msg = msg.as_ref();
        let [signature @ .., v] = signature;

        let public_key = VerifyingKey::from_encoded_point(&EncodedPoint::from_untagged_bytes(
            &public_key.into(),
        ))
        .unwrap();
        let signature = Signature::from_bytes(&signature.into()).unwrap();
        let recovery_id = RecoveryId::from_byte(v).unwrap();

        assert_eq!(
            Tip191::recover(msg, &signature, recovery_id),
            Some(public_key),
            "invalid recovered public key",
        );
    }
}
