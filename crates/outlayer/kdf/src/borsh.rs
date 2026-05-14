use std::marker::PhantomData;

use borsh::BorshSerialize;

use crate::{Identity, SubScheme};

pub struct Borsh<S: ?Sized = Identity>(PhantomData<S>);

impl<S, P> SubScheme<P> for Borsh<S>
where
    S: SubScheme<P>,
    S::Output: BorshSerialize,
{
    type Output = Vec<u8>;

    fn derive(path: P) -> Self::Output {
        let path = S::derive(path);
        
        borsh::to_vec(&path).expect("borsh")
    }
}
