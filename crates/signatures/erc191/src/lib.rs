use defuse_digest::{Digest, sha3::Keccak256};
use defuse_kdf_crypto::Secp256k1;
use defuse_signature_schema::{Result, Schema, SignatureSchema};

/// [ERC-191](https://eips.ethereum.org/EIPS/eip-191) Signed Data Standard:
///
/// ```text
/// 0x19 <0x45 (E)> <thereum Signed Message:\n" + len(message)> <data to sign>
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Erc191;

impl<M> Schema<M> for Erc191
where
    M: AsRef<[u8]>,
{
    type Output = [u8; 32];

    /// Derive prehash for signing
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hex_literal::hex;
    /// use defuse_erc191::Erc191;
    /// use defuse_signature_schema::Schema;
    ///
    /// assert_eq!(
    ///     Erc191.derive("Hello world!").unwrap(),
    ///     hex!("aa05af77f274774b8bdc7b61d98bc40da523dc2821fdea555f4d6aa413199bcc"),
    /// );
    /// ```
    fn derive(&self, msg: M) -> Result<Self::Output> {
        thread_local! {
            // per-thread lazily-initialized hasher with pre-processed prefix
            static HASHER: Keccak256 = Keccak256::new_with_prefix(b"\x19Ethereum Signed Message:\n");
        }

        let msg = msg.as_ref();

        Ok(HASHER
            .with(Clone::clone)
            // + len(message)
            .chain_update(msg.len().to_string())
            // <data to sign>
            .chain_update(msg)
            .finalize()
            .into())
    }
}

impl<M> SignatureSchema<M> for Erc191
where
    M: AsRef<[u8]>,
{
    type Curve = Secp256k1;
}

#[cfg(test)]
mod tests {
    use defuse_kdf_crypto::k256::{
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
    fn verify_ok(
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

        assert!(Erc191.verify(&public_key, msg, &signature).unwrap());

        assert_eq!(
            Erc191.recover(msg, &signature, recovery_id).unwrap(),
            Some(public_key)
        );
    }
}
