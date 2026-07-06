use std::{marker::PhantomData, rc::Rc, sync::Arc};

pub use defuse_derivation_schema::*;

use defuse_kdf_crypto::{Curve, RecoverableCurve};
use impl_tools::autoimpl;

#[autoimpl(for<S: trait + ?Sized> &S, &mut S, Box<S>, Rc<S>, Arc<S>)]
pub trait SignatureSchema<M>: Schema<M, Output: AsRef<[u8]>> {
    type Curve: Curve;

    #[inline]
    fn verify(
        &self,
        public_key: &<Self::Curve as Curve>::PublicKey,
        msg: M,
        signature: &<Self::Curve as Curve>::Signature,
    ) -> Result<bool> {
        let msg = self.derive(msg)?;
        Ok(Self::Curve::verify(public_key, msg.as_ref(), signature))
    }

    #[inline]
    fn recover(
        &self,
        msg: M,
        signature: &<Self::Curve as Curve>::Signature,
        recovery_id: <Self::Curve as RecoverableCurve>::RecoveryId,
    ) -> Result<Option<<Self::Curve as Curve>::PublicKey>>
    where
        Self::Curve: RecoverableCurve,
    {
        let msg = self.derive(msg)?;
        Ok(Self::Curve::recover(msg.as_ref(), signature, recovery_id))
    }
}

#[autoimpl(Debug, Clone, Copy, Default)]
pub struct Raw<C: Curve>(PhantomData<C>);

impl<C, M> Schema<M> for Raw<C>
where
    C: Curve,
    M: AsRef<[u8]>,
{
    type Output = M;

    #[inline]
    fn derive(&self, input: M) -> Result<Self::Output> {
        Ok(input)
    }
}

impl<C, M> SignatureSchema<M> for Raw<C>
where
    C: Curve,
    M: AsRef<[u8]>,
{
    type Curve = C;
}
