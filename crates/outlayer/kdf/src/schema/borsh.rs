use borsh::BorshSerialize;

use crate::{DerivableCurve, DerivationSchema};

#[derive(Default)]
pub struct Borsh;

impl<C, P> DerivationSchema<C, P> for Borsh
where
    C: DerivableCurve,
    P: BorshSerialize,
{
    type Output = Vec<u8>;

    fn derive_path(&self, path: P) -> Self::Output {
        borsh::to_vec(&path).expect("borsh")
    }
}
