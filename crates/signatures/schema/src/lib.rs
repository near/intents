use std::{marker::PhantomData, rc::Rc, sync::Arc};

pub use defuse_derivation_schema::*;

use defuse_crypto::{Curve, RecoverableCurve};
use impl_tools::autoimpl;

// pub trait SignatureSchema1<M> {
//     type PublicKey;
//     type Signature;

//     fn verify(&self, public_key: &Self::PublicKey, msg: M, signature: &Self::Signature) -> bool;
// }

#[autoimpl(for<S: trait + ?Sized> &S, &mut S, Box<S>, Rc<S>, Arc<S>)]
pub trait SignatureSchema<M>: Schema<M, Output: AsRef<[u8]>> {
    // TODO: having one public-key/signature makes it impossible
    // to implement synchronous multi-sig...
    // However, multi-sig members might want to use same signature scheme.
    // Are they? What if I want to implement a `co-signer` that just signs my signature?...
    // Should multi-sigs be implemented on-top of `SignatureSchema`?
    // TODO: this also doesn't support `0x123abc...` as public key
    type Curve: Curve;
    // type SignerData;

    // fn check_derive(&self, msg: M, signer_data: &Self::SignerData) -> Result<impl AsRef<[u8]>>;

    // verify:
    // 1. self - only for algorithm params
    // 2. message - would be nice if composable
    // 3. SignerData
    // 4. Signature - can't change to recoverable, serialization format fixed
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
