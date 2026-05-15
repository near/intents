use borsh::BorshSerialize;

use crate::{DerivableCurve, DerivationSchema, DeriveSigner, Identity};

pub struct Borsh<S = Identity>(S);

impl<C, S, P> DerivationSchema<C, P> for Borsh<S>
where
    C: DerivableCurve,
    P: BorshSerialize,
    S: DerivationSchema<C, Vec<u8>>,
{
    type Output = S::Output;

    fn derive_path(&self, path: P) -> Self::Output {
        self.0.derive_path(borsh::to_vec(&path).expect("borsh"))
    }
}

impl<C, S, P> DeriveSigner<C, P> for Borsh<S>
where
    C: DerivableCurve,
    P: BorshSerialize,
    S: DeriveSigner<C, Vec<u8>>,
{
    fn public_key(&self) -> C::PublicKey {
        self.0.public_key()
    }

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature {
        self.0
            .derive_sign(borsh::to_vec(&path).expect("borsh"), msg)
    }
}
