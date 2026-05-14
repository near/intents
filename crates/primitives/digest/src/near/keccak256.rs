use digest::{FixedOutput, HashMarker, OutputSizeUser, Update, consts::U32};
use near_sdk::env;

#[derive(Debug, Clone, Default)]
pub struct Keccak256 {
    data: Vec<u8>,
}

impl Update for Keccak256 {
    #[inline]
    fn update(&mut self, data: &[u8]) {
        self.data.extend(data);
    }
}

impl OutputSizeUser for Keccak256 {
    type OutputSize = U32;
}

impl FixedOutput for Keccak256 {
    #[inline]
    fn finalize_into(self, out: &mut digest::Output<Self>) {
        *out = self.finalize_fixed();
    }

    #[inline]
    fn finalize_fixed(self) -> digest::Output<Self> {
        env::keccak256_array(&self.data).into()
    }
}

impl HashMarker for Keccak256 {}

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
}
