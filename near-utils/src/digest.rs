use digest::{FixedOutput, HashMarker, OutputSizeUser, Update, consts::{U32, U20}};
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

/// NEAR SDK HASH160 implementation compatible with the `digest` crate traits.
///
/// HASH160 is Bitcoin's standard address hash function: RIPEMD160(SHA256(data)).
/// This implementation uses NEAR SDK's host functions for optimal gas efficiency.
#[derive(Debug, Clone, Default)]
pub struct Hash160 {
    data: Vec<u8>,
}

impl Update for Hash160 {
    #[inline]
    fn update(&mut self, data: &[u8]) {
        self.data.extend(data);
    }
}

impl OutputSizeUser for Hash160 {
    type OutputSize = U20;
}

impl FixedOutput for Hash160 {
    #[inline]
    fn finalize_into(self, out: &mut digest::Output<Self>) {
        *out = self.finalize_fixed();
    }

    #[inline]
    fn finalize_fixed(self) -> digest::Output<Self> {
        // First pass: SHA256 using NEAR SDK host function
        let sha256_result = env::sha256_array(&self.data);
        // Second pass: RIPEMD160 using NEAR SDK host function
        env::ripemd160_array(&sha256_result).into()
    }
}

impl HashMarker for Hash160 {}

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

    #[rstest]
    fn hash160_digest(random_bytes: Vec<u8>) {
        let got: [u8; 20] = Hash160::digest(&random_bytes).into();
        let expected = {
            let sha256_result = env::sha256_array(&random_bytes);
            env::ripemd160_array(&sha256_result)
        };
        assert_eq!(got, expected);
    }
}
