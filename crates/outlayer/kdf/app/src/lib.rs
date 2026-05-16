use borsh::BorshSerialize;
use defuse_outlayer_kdf::Map;
pub use defuse_outlayer_kdf::{self as kdf, DerivableCurve, DerivationSchema, DeriveSigner};
pub use defuse_outlayer_primitives::AppId;
use digest_io::IoWrapper;
use sha3::{Digest, Sha3_256};

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[derive(BorshSerialize)]
/// **Non-hierarchical** derivation path
struct AppDerivationPath<'a> {
    /// Identifier of an application to derive for
    pub app_id: AppId<'a>,
    /// Application-level path
    pub path: &'a str,
}

pub struct AppDerivation<'a>(AppId<'a>);

impl<'a> AppDerivation<'a> {
    #[inline]
    pub const fn new(app_id: AppId<'a>) -> Self {
        Self(app_id)
    }
}

impl<P> DerivationSchema<P> for AppDerivation<'_>
where
    P: AsRef<str>,
{
    type Output = [u8; 32];

    fn derive_path(&self, path: P) -> Self::Output {
        const DOMAIN_SEPARATOR: &[u8] = b"outlayer/app-derivation/v1";

        let path = AppDerivationPath {
            app_id: self.0.as_ref(),
            path: path.as_ref(),
        };

        let mut hasher = IoWrapper(Sha3_256::new_with_prefix(DOMAIN_SEPARATOR));
        // serialize directly to hasher
        borsh::to_writer(&mut hasher, &path).expect("borsh");
        hasher.0.finalize().into()
    }
}

pub struct AppSigner<'a, S> {
    app_id: AppId<'a>,
    signer: S,
}

impl<'a, S> AppSigner<'a, S> {
    #[inline]
    pub const fn new(app_id: AppId<'a>, signer: S) -> Self {
        Self { app_id, signer }
    }
}

impl<C, P, S> DeriveSigner<C, P> for AppSigner<'_, S>
where
    C: DerivableCurve,
    P: AsRef<str>,
    S: DeriveSigner<C, [u8; 32]>,
{
    type Schema<'a>
        = Map<AppDerivation<'a>, S::Schema<'a>>
    where
        Self: 'a;

    fn schema(&self) -> Self::Schema<'_> {
        // TODO
        Map::new(
            AppDerivation::new(self.app_id.as_ref()),
            self.signer.schema(),
        )
    }

    fn public_key(&self) -> C::PublicKey {
        self.signer.public_key()
    }

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
        let schema = AppDerivation::new(self.app_id.as_ref());
        let path = schema.derive_path(path);
        self.signer.derive_sign(path, msg)
    }
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        "near:test.near",
        "",
        hex!("f2ce50c4a56ffb40c9a7f60f6b3d677b92cbae4aae3311356b3048ce247acc50"),
    )]
    #[case(
        "near:test.near",
        "test",
        hex!("d0d1b75f072ded2c6a495e2179f6d064e71c4d47ad909678b0ef0fb56ced2a56"),
    )]
    #[case(
        "near:0s1234567890abcdef1234567890abcdef12345678",
        "test",
        hex!("93f51d34d69d988672ae0e979d96c1a7dec4239f2819839db89bbfaa2dbcc668"),
    )]
    fn derive_has_not_changed(
        #[case] app_id: &str,
        #[case] path: &str,
        #[case] expected: [u8; 32],
    ) {
        let schema = AppDerivation::new(app_id.parse().expect("invalid app_id"));

        assert_eq!(
            schema.derive_path(path),
            expected,
            "derived hash has changed"
        );
    }
}
