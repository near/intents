//! [SEP-53](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0053.md) Signed Data Standard

use defuse_crypto::{Curve, ed25519::Ed25519};
use defuse_digest::{Digest, sha2::Sha256};

/// [SEP-53](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0053.md) Signed Data Standard
pub struct Sep53;

impl Sep53 {
    /// Verify signature over a given message for given public key according to
    /// [SEP-53](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0053.md).
    #[must_use = "check if verification passed"]
    #[inline]
    pub fn verify(
        public_key: &<Ed25519 as Curve>::PublicKey,
        msg: impl AsRef<[u8]>,
        signature: &<Ed25519 as Curve>::Signature,
    ) -> bool {
        Ed25519::verify(public_key, &Self::prehash(msg.as_ref()), signature)
    }

    /// Derive prehash for signing according to following schema:
    ///
    /// ```text
    /// <"Stellar Signed Message:\n"> <data to sign>
    /// ```
    #[inline]
    pub fn prehash(msg: impl AsRef<[u8]>) -> [u8; 32] {
        Sha256::new_with_prefix(b"Stellar Signed Message:\n")
            // <data to sign>
            .chain_update(msg)
            .finalize()
            .into()
    }
}

#[cfg(test)]
mod tests {
    use defuse_crypto::ed25519::ed25519_dalek::{SIGNATURE_LENGTH, Signature, VerifyingKey};
    use hex_literal::hex;
    use rstest::rstest;
    use stellar_strkey::Strkey;

    use super::*;

    // https://github.com/stellar/stellar-protocol/blob/1b1c22e02fc0cec6fff1175c2d7d08ad83a828e1/ecosystem/sep-0053.md#test-cases
    #[rstest]
    #[case::ascii(
        "GBXFXNDLV4LSWA4VB7YIL5GBD7BVNR22SGBTDKMO2SBZZHDXSKZYCP7L",
        "Hello, World!",
        hex!("7cee5d6d885752104c85eea421dfdcb95abf01f1271d11c4bec3fcbd7874dccd6e2e98b97b8eb23b643cac4073bb77de5d07b0710139180ae9f3cbba78f2ba04"),
    )]
    #[case::japanese(
        "GBXFXNDLV4LSWA4VB7YIL5GBD7BVNR22SGBTDKMO2SBZZHDXSKZYCP7L",
        "こんにちは、世界！",
        hex!("083536eb95ecf32dce59b07fe7a1fd8cf814b2ce46f40d2a16e4ea1f6cecd980e04e6fbef9d21f98011c785a81edb85f3776a6e7d942b435eb0adc07da4d4604"),
    )]
    #[case::binary(
        "GBXFXNDLV4LSWA4VB7YIL5GBD7BVNR22SGBTDKMO2SBZZHDXSKZYCP7L",
        hex!("db36433f5b1ad415417cb3fb4de78c937b146dac4091484184388d76b92c685a"),
        hex!("540d7eee179f370bf634a49c1fa9fe4a58e3d7990b0207be336c04edfcc539ff8bd0c31bb2c0359b07c9651cb2ae104e4504657b5d17d43c69c7e50e23811b0d"),
    )]
    fn verify_ok(
        #[case] address: &str,
        #[case] msg: impl AsRef<[u8]>,
        #[case] signature: [u8; SIGNATURE_LENGTH],
    ) {
        let Strkey::PublicKeyEd25519(stellar_strkey::ed25519::PublicKey(public_key)) =
            address.parse().unwrap()
        else {
            panic!("invalid ed25519 public key address");
        };

        let public_key = VerifyingKey::from_bytes(&public_key).unwrap();
        let signature = Signature::from_bytes(&signature);

        assert!(
            Sep53::verify(&public_key, msg, &signature),
            "invalid signature",
        );
    }
}
