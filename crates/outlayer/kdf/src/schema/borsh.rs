use borsh::BorshSerialize;

use crate::DerivationSchema;

#[derive(Default)]
pub struct Borsh;

impl<P> DerivationSchema<P> for Borsh
where
    P: BorshSerialize,
{
    type Output = Vec<u8>;

    fn derive_path(&self, path: P) -> Self::Output {
        borsh::to_vec(&path).expect("borsh")
    }
}
