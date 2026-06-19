use std::marker::PhantomData;

use digest::{FixedOutput, HashMarker, Output, OutputSizeUser, Update};
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

#[cfg(feature = "sha2")]
pub mod sha2 {
    use digest::consts::U32;

    use super::*;

    pub type Sha256 = EnvDigest<Sha256Fn>;

    pub struct Sha256Fn;

    impl OutputSizeUser for Sha256Fn {
        type OutputSize = U32;
    }

    impl DigestFn for Sha256Fn {
        fn digest(bytes: &[u8]) -> Output<Self> {
            near_sdk_env::sha256_array(bytes).into()
        }
    }
}

#[cfg(feature = "sha3")]
pub mod sha3 {
    use digest::consts::{U32, U64};

    use super::*;

    pub type Keccak256 = EnvDigest<Keccak256Fn>;
    pub type Keccak512 = EnvDigest<Keccak512Fn>;

    pub struct Keccak256Fn;

    impl OutputSizeUser for Keccak256Fn {
        type OutputSize = U32;
    }

    impl DigestFn for Keccak256Fn {
        fn digest(bytes: &[u8]) -> Output<Self> {
            near_sdk_env::keccak256_array(bytes).into()
        }
    }

    pub struct Keccak512Fn;

    impl OutputSizeUser for Keccak512Fn {
        type OutputSize = U64;
    }

    impl DigestFn for Keccak512Fn {
        fn digest(bytes: &[u8]) -> Output<Self> {
            near_sdk_env::keccak512_array(bytes).into()
        }
    }
}
