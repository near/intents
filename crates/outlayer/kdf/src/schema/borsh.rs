use borsh::BorshSerialize;

use crate::{DerivableCurve, DerivationSchema, Identity};

pub struct Borsh<S = Identity>(S);

impl<C, S, P> DerivationSchema<C, P> for Borsh<S>
where
    C: DerivableCurve + ?Sized,
    P: BorshSerialize,
    S: DerivationSchema<C, Vec<u8>>,
{
    type Output = S::Output;

    fn derive(&self, path: P) -> Self::Output {
        self.0.derive(borsh::to_vec(&path).expect("borsh"))
    }
}
