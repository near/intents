use borsh::BorshSerialize;
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

impl<C, P> DerivationSchema<C, P> for AppDerivation<'_>
where
    C: DerivableCurve,
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

impl<C, P, S> DerivationSchema<C, P> for AppSigner<'_, S>
where
    C: DerivableCurve,
    P: AsRef<str>,
    S: DerivationSchema<C, [u8; 32]>,
{
    type Output = S::Output;

    fn derive_path(&self, path: P) -> Self::Output {
        let schema = AppDerivation::new(self.app_id.as_ref());
        let path = DerivationSchema::<C, _>::derive_path(&schema, path);
        self.signer.derive_path(path)
    }
}

impl<C, P, S> DeriveSigner<C, P> for AppSigner<'_, S>
where
    C: DerivableCurve,
    P: AsRef<str>,
    S: DeriveSigner<C, [u8; 32]>,
{
    fn public_key(&self) -> C::PublicKey {
        self.signer.public_key()
    }

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
        let schema = AppDerivation::new(self.app_id.as_ref());
        let path = DerivationSchema::<C, _>::derive_path(&schema, path);
        self.signer.derive_sign(path, msg)
    }
}
