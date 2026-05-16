use borsh::BorshSerialize;
use defuse_kdf::DerivationSchema;
use defuse_outlayer_primitives::AppId;
use digest_io::IoWrapper;
use sha3::{Digest, Sha3_256};

const DOMAIN_SEPARATOR: &[u8] = b"outlayer/app-derivation/v1";
thread_local! {
    // per-thread lazily-initialized hasher with pre-processed domain separator
    static HASHER: Sha3_256 = Sha3_256::new_with_prefix(DOMAIN_SEPARATOR);
}

/// [`DerivationSchema`] for Outlayer applications
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AppDerivation<'a>(AppId<'a>);

impl<'a> AppDerivation<'a> {
    #[inline]
    pub const fn new(app_id: AppId<'a>) -> Self {
        Self(app_id)
    }

    #[inline]
    pub fn app_id(&self) -> AppId<'_> {
        self.0.as_ref()
    }
}

#[derive(BorshSerialize)]
struct AppDerivationPath<'a> {
    /// Identifier of an application to derive for
    pub app_id: AppId<'a>,
    /// Application-level path
    pub path: &'a str,
}

impl<'a, P> DerivationSchema<P> for AppDerivation<'a>
where
    P: AsRef<str>,
{
    type Output = [u8; 32];

    fn derive_path(&self, path: P) -> Self::Output {
        let path = AppDerivationPath {
            app_id: self.0.as_ref(),
            path: path.as_ref(),
        };

        // serialize directly to hasher
        let mut hasher = IoWrapper(HASHER.with(Clone::clone));
        borsh::to_writer(&mut hasher, &path).expect("borsh");
        hasher.0.finalize().into()
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
        let app_id = app_id.parse().expect("invalid app_id");

        let schema = AppDerivation::new(app_id);

        assert_eq!(
            schema.derive_path(path),
            expected,
            "derived hash has changed"
        );
    }
}
