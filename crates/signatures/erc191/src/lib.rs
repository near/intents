//! [ERC-191](https://eips.ethereum.org/EIPS/eip-191) Signed Data Standard

use defuse_crypto::{Curve, RecoverableCurve, secp256k1::Secp256k1};
use defuse_digest::{Digest, sha3::Keccak256};

/// [ERC-191](https://eips.ethereum.org/EIPS/eip-191) Signed Data Standard
pub struct Erc191;

impl Erc191 {
    /// Try to recover public key which signed given message according to
    /// [ERC-191](https://eips.ethereum.org/EIPS/eip-191) and produced given
    /// signature and recovery id.
    #[must_use = "check recovered public key"]
    #[inline]
    pub fn recover(
        msg: impl AsRef<[u8]>,
        signature: &<Secp256k1 as Curve>::Signature,
        recovery_id: <Secp256k1 as RecoverableCurve>::RecoveryId,
    ) -> Option<<Secp256k1 as Curve>::PublicKey> {
        Secp256k1::recover(&Self::prehash(msg), signature, recovery_id)
    }

    /// Derive prehash for signing according to following schema:
    ///
    /// ```text
    /// 0x19 <0x45 (E)> <thereum Signed Message:\n" + len(message)> <data to sign>
    /// ```
    #[inline]
    pub fn prehash(msg: impl AsRef<[u8]>) -> [u8; 32] {
        let msg = msg.as_ref();

        Keccak256::new_with_prefix(b"\x19Ethereum Signed Message:\n")
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
        "Hello world!",
        hex!("7800a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e4701"),
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
            Erc191::recover(msg, &signature, recovery_id),
            Some(public_key),
            "invalid recovered public key",
        );
    }
}
