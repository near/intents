use digest::{FixedOutput, HashMarker, OutputSizeUser, Update};

#[derive(Debug, Clone, Default)]
pub struct Double<D>(D);

impl<D> Update for Double<D>
where
    D: Update,
{
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
}

impl<D> OutputSizeUser for Double<D>
where
    D: OutputSizeUser,
{
    type OutputSize = D::OutputSize;
}

impl<D> FixedOutput for Double<D>
where
    D: FixedOutput + Update + Default,
{
    fn finalize_into(self, out: &mut digest::Output<Self>) {
        D::default()
            .chain(self.0.finalize_fixed())
            .finalize_into(out);
    }
}

impl<D> HashMarker for Double<D> where D: HashMarker {}

#[cfg(test)]
mod tests {
    use digest::Update;
    use hex_literal::hex;
    use rstest::rstest;
    use sha2::{Digest, Sha256, Sha512};

    use super::*;

    /// Test Double<Sha256> with various inputs
    #[rstest]
    #[case(b"", hex!("5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456"))]
    #[case(b"hello", hex!("9595c9df90075148eb06860365df33584b75bff782a510c6cd4883a419833d50"))]
    fn test_double_sha256_vectors(#[case] input: &[u8], #[case] expected: [u8; 32]) {
        let result = Double::<Sha256>::digest(input);
        assert_eq!(result.as_slice(), &expected);
    }

    /// Test Double<Sha256> with additional test cases (computed dynamically)
    #[test]
    fn test_double_sha256_additional_cases() {
        let test_cases = [
            b"bitcoin".as_slice(),
            b"The Times 03/Jan/2009 Chancellor on brink of second bailout for banks".as_slice(),
            &[0u8; 32],
            &[0xffu8; 64],
        ];

        for input in test_cases {
            let result = Double::<Sha256>::digest(input);

            // Verify by computing manually
            let first_hash = Sha256::digest(input);
            let expected = Sha256::digest(first_hash);

            assert_eq!(result.as_slice(), expected.as_slice());
        }
    }

    /// Test that Double<D> works with different digest types
    #[test]
    fn test_double_sha512() {
        let input = b"test";
        let result = Double::<Sha512>::digest(input);

        // Verify it's double hashing by computing manually
        let first_hash = Sha512::digest(input);
        let expected = Sha512::digest(first_hash);

        assert_eq!(result.as_slice(), expected.as_slice());
        assert_eq!(result.len(), 64); // SHA512 output size
    }

    /// Test incremental hashing with Double<D>
    #[test]
    fn test_double_incremental_hashing() {
        let data1 = b"hello";
        let data2 = b"world";

        // Hash incrementally
        let mut hasher = Double::<Sha256>::new();
        Update::update(&mut hasher, data1);
        Update::update(&mut hasher, data2);
        let incremental_result = hasher.finalize();

        // Hash all at once
        let mut combined = Vec::new();
        combined.extend_from_slice(data1);
        combined.extend_from_slice(data2);
        let direct_result = Double::<Sha256>::digest(&combined);

        assert_eq!(incremental_result, direct_result);
    }

    /// Test that Double<D> produces different results than single hash
    #[test]
    fn test_double_vs_single_hash() {
        let input = b"bitcoin";

        let single_hash = Sha256::digest(input);
        let double_hash = Double::<Sha256>::digest(input);

        // They should be different
        assert_ne!(single_hash.as_slice(), double_hash.as_slice());

        // Double hash should equal manually computed double hash
        let manual_double = Sha256::digest(single_hash);
        assert_eq!(double_hash.as_slice(), manual_double.as_slice());
    }

    /// Test empty input edge case
    #[test]
    fn test_double_empty_input() {
        let empty_input = b"";
        let result = Double::<Sha256>::digest(empty_input);

        // Should not panic and should produce deterministic result
        assert_eq!(result.len(), 32);

        // Verify it matches the expected empty string double SHA256
        let expected = hex!("5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456");
        assert_eq!(result.as_slice(), &expected);
    }

    /// Test large input handling
    #[test]
    fn test_double_large_input() {
        // Create a 1MB input
        let large_input = vec![0x42u8; 1024 * 1024];
        let result = Double::<Sha256>::digest(&large_input);

        // Should handle large inputs without issues
        assert_eq!(result.len(), 32);

        // Should be deterministic
        let result2 = Double::<Sha256>::digest(&large_input);
        assert_eq!(result, result2);
    }

    /// Test trait implementations work correctly
    #[test]
    fn test_double_trait_implementations() {
        let mut hasher = Double::<Sha256>::new();

        // Test Update trait
        Update::update(&mut hasher, b"test");
        Update::update(&mut hasher, b"data");

        let result = hasher.finalize();
        assert_eq!(result.len(), 32);

        // Test Default trait
        let default_hasher = Double::<Sha256>::default();
        let empty_result = default_hasher.finalize();
        let expected_empty = Double::<Sha256>::digest(b"");
        assert_eq!(empty_result, expected_empty);
    }

    /// Test multiple updates produce same result as single update
    #[test]
    fn test_double_multiple_updates() {
        let data = b"The quick brown fox jumps over the lazy dog";

        // Single update
        let single_result = Double::<Sha256>::digest(data);

        // Multiple updates (split the data)
        let mut hasher = Double::<Sha256>::new();
        for chunk in data.chunks(5) {
            Update::update(&mut hasher, chunk);
        }
        let multiple_result = hasher.finalize();

        assert_eq!(single_result, multiple_result);
    }
}
