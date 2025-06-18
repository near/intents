use digest::{FixedOutput, HashMarker, OutputSizeUser, Update, consts::U32};
use near_sdk::env;

#[derive(Debug, Clone, Default)]
pub struct Sha256 {
    data: Vec<u8>,
}

impl Update for Sha256 {
    #[inline]
    fn update(&mut self, data: &[u8]) {
        self.data.extend(data);
    }
}

impl OutputSizeUser for Sha256 {
    type OutputSize = U32;
}

impl FixedOutput for Sha256 {
    #[inline]
    fn finalize_into(self, out: &mut digest::Output<Self>) {
        *out = self.finalize_fixed();
    }

    #[inline]
    fn finalize_fixed(self) -> digest::Output<Self> {
        env::sha256_array(&self.data).into()
    }
}

impl HashMarker for Sha256 {}

#[cfg(test)]
mod tests {
    use defuse_test_utils::random::random_bytes;
    use digest::Digest;
    use near_sdk::CryptoHash;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn digest(random_bytes: Vec<u8>) {
        let got: CryptoHash = Sha256::digest(&random_bytes).into();
        assert_eq!(got, env::sha256_array(&random_bytes));
    }
}
