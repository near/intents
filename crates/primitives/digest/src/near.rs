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
pub struct NearKeccak256;

impl OutputSizeUser for NearKeccak256 {
    type OutputSize = U32;
}

impl DigestFn for NearKeccak256 {
    fn digest(bytes: &[u8]) -> Output<Self> {
        env::keccak256_array(bytes).into()
    }
}

pub type Keccak256 = VecDigest<NearKeccak256>;

#[derive(Default, Clone)]
pub struct NearSha256;

impl OutputSizeUser for NearSha256 {
    type OutputSize = U32;
}

impl DigestFn for NearSha256 {
    fn digest(bytes: &[u8]) -> Output<Self> {
        env::sha256_array(bytes).into()
    }
}

pub type Sha256 = VecDigest<NearSha256>;

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
