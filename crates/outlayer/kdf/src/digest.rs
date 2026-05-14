use std::marker::PhantomData;

use digest::Output;

use crate::{Identity, SubScheme};

pub struct Digest<D, S: ?Sized = Identity>(PhantomData<D>, PhantomData<S>);

impl<D, S, P> SubScheme<P> for Digest<D, S>
where
    S: SubScheme<P> + ?Sized,
    S::Output: AsRef<[u8]>,
    D: digest::Digest,
{
    type Output = Output<D>;

    fn derive(path: P) -> Self::Output {
        let path = S::derive(path);

        D::digest(path)
    }
}
