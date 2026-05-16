pub use defuse_outlayer_kdf::ed25519::*;

use defuse_outlayer_kdf::{Curve, DerivationSchema, DeriveSigner};

use crate::{DomainCurve, InMemorySigner, Schema, sealed::Sealed};

impl<P> DeriveSigner<Ed25519, P> for InMemorySigner
where
    P: AsRef<[u8]>,
{
    type Schema<'a>
        = Schema<Ed25519>
    where
        Self: 'a;

    fn schema(&self) -> Self::Schema<'_> {
        Schema::default()
    }

    fn public_key(&self) -> <Ed25519 as Curve>::PublicKey {
        self.ed25519_master_sk.public_key()
    }

    fn derive_sign(&self, path: P, msg: &[u8]) -> <Ed25519 as Curve>::Signature {
        let tweak = DeriveSigner::<Ed25519, _>::derive_tweak(self, path);
        self.ed25519_master_sk.derive_sign(tweak, msg)
    }
}

#[derive(Default)]
pub struct FromBytesModOrder;

impl DerivationSchema<[u8; 32]> for FromBytesModOrder {
    type Output = Scalar;

    fn derive_path(&self, path: [u8; 32]) -> Self::Output {
        Scalar::from_bytes_mod_order(path)
    }
}

impl DomainCurve for Ed25519 {
    const DOMAIN_SEPARATOR: &[u8] = b"outlayer/ed25519/derive-tweak/v1";

    type ToTweak = FromBytesModOrder;
}

impl Sealed for Ed25519 {}

#[cfg(test)]
mod tests {
    use defuse_outlayer_kdf::DerivationSchema;
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        b"",
        hex!("3b23f008ada264aa8d80cb56d3e7b852bd87956fe0fee6d57315ae021f782d0e")
    )]
    #[case(
        b"test",
        hex!("82e90e99953d32d97d98affe0d5e41e28e6253d4511ab2f4662c4d6bb6b5470d")
    )]
    #[case(
        b"5b74d49e83f4aff956284d5f74270e53d7c55dabc4c28f6ef923fbffc5bfdd1d",
        hex!("a64f77d0cba6207685fb450bd0a409815489c1d31c4b6165fc0d060b29faaf08")
    )]
    fn tweak_derivation_schema_has_not_changed(
        #[case] path: impl AsRef<[u8]>,
        #[case] expected_tweak: [u8; 32],
    ) {
        let got = Schema::<Ed25519>::default().derive_path(path);

        assert_eq!(got.to_bytes(), expected_tweak, "derived tweak has changed");
    }
}
