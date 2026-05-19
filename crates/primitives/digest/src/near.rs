use std::marker::PhantomData;

use digest::{FixedOutput, HashMarker, Output, OutputSizeUser, Update, consts::U32};
use near_sdk::env;

trait DigestFn: OutputSizeUser {
    fn digest(bytes: &[u8]) -> Output<Self>;
}

#[derive(Default, Clone)]
pub struct VecDigest<F> {
    data: Vec<u8>,
    _f: PhantomData<F>,
}

impl<F: OutputSizeUser> OutputSizeUser for VecDigest<F> {
    type OutputSize = F::OutputSize;
}

impl<F> Update for VecDigest<F> {
    #[inline]
    fn update(&mut self, data: &[u8]) {
        self.data.extend(data);
    }
}

impl<F: DigestFn> FixedOutput for VecDigest<F> {
    #[inline]
    fn finalize_into(self, out: &mut digest::Output<Self>) {
        *out = F::digest(&self.data);
    }
}

impl<F: DigestFn> HashMarker for VecDigest<F> {}

#[derive(Default, Clone)]
pub struct Keccak256Fn;

impl OutputSizeUser for Keccak256Fn {
    type OutputSize = U32;
}

impl DigestFn for Keccak256Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        env::keccak256_array(bytes).into()
    }
}

pub type Keccak256 = VecDigest<Keccak256Fn>;

#[derive(Default, Clone)]
pub struct Sha256Fn;

impl OutputSizeUser for Sha256Fn {
    type OutputSize = U32;
}

impl DigestFn for Sha256Fn {
    fn digest(bytes: &[u8]) -> Output<Self> {
        env::sha256_array(bytes).into()
    }
}

pub type Sha256 = VecDigest<Sha256Fn>;

#[cfg(test)]
mod tests {
    use defuse_test_utils::random::random_bytes;
    use digest::Digest;
    use near_sdk::CryptoHash;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn keccak256(random_bytes: Vec<u8>) {
        let got: CryptoHash = Keccak256::digest(&random_bytes).into();
        assert_eq!(got, env::keccak256_array(&random_bytes));
    }

    #[rstest]
    fn sha256(random_bytes: Vec<u8>) {
        let got: CryptoHash = Sha256::digest(&random_bytes).into();
        assert_eq!(got, env::sha256_array(&random_bytes));
    }
}
