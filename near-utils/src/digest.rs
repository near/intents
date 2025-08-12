use digest::{Digest, FixedOutput, HashMarker, OutputSizeUser, Update, consts::{U32, U20}};
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

/// Double digest wrapper that applies a hash function twice.
///
/// This is commonly used in Bitcoin protocols where double SHA-256 is the standard.
/// The algorithm: `Hash(Hash(data))`
///
/// This is a generic wrapper that works with any digest implementing the required traits.
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

/// Tagged digest trait for domain-separated hashing.
///
/// Tagged hashing prevents signature reuse across different contexts by 
/// domain-separating the hash computation with a tag.
///
/// The algorithm: `Hash(tag_hash || tag_hash || data)` where `tag_hash = Hash(tag)`
///
/// This is used in BIP-340 (Schnorr signatures) and BIP-322 (message signatures).
pub trait TaggedDigest: Digest {
    fn tagged(tag: impl AsRef<[u8]>) -> Self;
}

impl<D: Digest> TaggedDigest for D {
    fn tagged(tag: impl AsRef<[u8]>) -> Self {
        let tag = Self::digest(tag);
        Self::new().chain_update(&tag).chain_update(&tag)
    }
}

/// Type alias for double SHA-256 using NEAR SDK functions.
///
/// Commonly used in Bitcoin protocols for transaction IDs, block hashes, and checksums.
pub type DoubleSha256 = Double<Sha256>;

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

    #[rstest]
    fn double_sha256_digest(random_bytes: Vec<u8>) {
        let got: [u8; 32] = DoubleSha256::digest(&random_bytes).into();
        let expected = {
            let first_hash = env::sha256_array(&random_bytes);
            env::sha256_array(&first_hash)
        };
        assert_eq!(got, expected);
    }

    #[rstest]
    fn tagged_digest_test(random_bytes: Vec<u8>) {
        let tag = b"test-tag";
        let got: [u8; 32] = Sha256::tagged(tag).chain_update(&random_bytes).finalize().into();
        
        let tag_hash = env::sha256_array(tag);
        let mut combined = Vec::with_capacity(tag_hash.len() * 2 + random_bytes.len());
        combined.extend_from_slice(&tag_hash);
        combined.extend_from_slice(&tag_hash);
        combined.extend_from_slice(&random_bytes);
        let expected = env::sha256_array(&combined);
        
        assert_eq!(got, expected);
    }
}
