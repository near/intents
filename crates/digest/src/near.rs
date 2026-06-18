use std::marker::PhantomData;

use digest::{FixedOutput, HashMarker, Output, OutputSizeUser, Update, consts::U32};
use impl_tools::autoimpl;

trait DigestFn: OutputSizeUser {
    fn digest(bytes: &[u8]) -> Output<Self>;
}

#[autoimpl(Default, Clone, PartialEq, Eq)]
pub struct EnvDigest<F> {
    data: Vec<u8>,
    _fn: PhantomData<F>,
}

impl<F: OutputSizeUser> OutputSizeUser for EnvDigest<F> {
    type OutputSize = F::OutputSize;
}

impl<F> Update for EnvDigest<F> {
    #[inline]
    fn update(&mut self, data: &[u8]) {
        self.data.extend(data);
    }
}

impl<F: DigestFn> FixedOutput for EnvDigest<F> {
    #[inline]
    fn finalize_into(self, out: &mut digest::Output<Self>) {
        *out = F::digest(&self.data);
    }
}

impl<F: DigestFn> HashMarker for EnvDigest<F> {}

pub type Keccak256 = EnvDigest<Keccak256Fn>;

#[derive(Default, Clone)]
pub struct Keccak256Fn;

impl OutputSizeUser for Keccak256Fn {
    type OutputSize = U32;
}

impl DigestFn for Keccak256Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::keccak256_array(bytes).into()
    }
}

pub type Sha256 = EnvDigest<Sha256Fn>;

#[derive(Default, Clone)]
pub struct Sha256Fn;

impl OutputSizeUser for Sha256Fn {
    type OutputSize = U32;
}

impl DigestFn for Sha256Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        near_sdk_env::sha256_array(bytes).into()
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use digest::{Digest, array::ArraySize, common::OutputSize};
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        Sha256Fn,
        b"",
        hex!("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
    )]
    #[case(
        Sha256Fn,
        b"test",
        hex!("9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"),
    )]
    #[case(
        Keccak256Fn,
        b"",
        hex!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"),
    )]
    #[case(
        Keccak256Fn,
        b"test",
        hex!("9c22ff5f21f0b81b113e63f7db6da94fedef11b2119b4088b89664fb9a3cb658"),
    )]
    fn has_not_changed<F>(
        #[case] _f: F,
        #[case] data: &[u8],
        #[case] output: <OutputSize<EnvDigest<F>> as ArraySize>::ArrayType<u8>,
    ) where
        F: DigestFn,
        <OutputSize<EnvDigest<F>> as ArraySize>::ArrayType<u8>: Debug + PartialEq,
    {
        assert_eq!(EnvDigest::<F>::digest(data).0, output, "has changed");
    }
}
