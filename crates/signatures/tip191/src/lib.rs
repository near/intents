use defuse_digest::{Digest, sha3::Keccak256};
use defuse_kdf_crypto::Secp256k1;
use defuse_signature_schema::{Result, Schema, SignatureSchema};

/// [TIP-191](https://github.com/tronprotocol/tips/blob/master/tip-191.md) Signed Data Standard
#[derive(Debug, Clone, Copy, Default)]
pub struct Tip191;

impl<M> Schema<M> for Tip191
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
    /// use defuse_tip191::Tip191;
    /// use defuse_signature_schema::Schema;
    ///
    /// assert_eq!(
    ///     Tip191.derive("Hello, TRON!").unwrap(),
    ///     hex!("1632c0ebba467e157675403ba3ba280b836e1801b5678d878dfc90bfc403d6e1"),
    /// );
    /// ```
    fn derive(&self, msg: M) -> Result<Self::Output> {
        thread_local! {
            // Prefix itself is not specified in the standard. But from: https://tronweb.network/docu/docs/Sign%20and%20Verify%20Message/
            //
            // per-thread lazily-initialized hasher with pre-processed prefix.
            static HASHER: Keccak256 = Keccak256::new_with_prefix(b"\x19TRON Signed Message:\n");
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

impl<M> SignatureSchema<M> for Tip191
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
        "Hello, TRON!",
        hex!("eea1651a60600ec4d9c45e8ae81da1a78377f789f0ac2019de66ad943459913015ef9256809ee0e6bb76e303a0b4802e475c1d26ade5d585292b80c9fe9cb10c01"),
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

        assert!(Tip191.verify(&public_key, msg, &signature).unwrap());

        assert_eq!(
            Tip191.recover(msg, &signature, recovery_id).unwrap(),
            Some(public_key)
        );
    }
}
