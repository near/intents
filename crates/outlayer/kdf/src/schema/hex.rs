use crate::DerivationSchema;

#[derive(Default)]
pub struct Hex;

impl<P> DerivationSchema<P> for Hex
where
    P: AsRef<[u8]>,
{
    type Output = String;

    fn derive_path(&self, path: P) -> Self::Output {
        hex::encode(path)
    }
}
