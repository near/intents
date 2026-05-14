use std::marker::PhantomData;

use crate::{Identity, SubScheme};

pub struct Hex<S: ?Sized = Identity>(PhantomData<S>);

impl<S, P> SubScheme<P> for Hex<S>
where
    S: SubScheme<P>,
    S::Output: AsRef<[u8]>,
{
    type Output = String;

    fn derive(path: P) -> Self::Output {
        let path = S::derive(path);

        hex::encode(path)
    }
}
