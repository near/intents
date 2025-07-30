use digest::Digest;

/// [BIP-340](https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki) tagged hash
pub trait Bip340TaggedDigest: Digest {
    fn tagged(tag: impl AsRef<[u8]>) -> Self;
}

impl<D: Digest> Bip340TaggedDigest for D {
    fn tagged(tag: impl AsRef<[u8]>) -> Self {
        let tag = Self::digest(tag);
        Self::new().chain_update(&tag).chain_update(&tag)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use sha2::Sha256;

    use super::*;

    #[rstest]
    fn sha256t(#[values(b"tag")] tag: &[u8], #[values(b"data")] data: &[u8]) {
        assert_eq!(
            Sha256::tagged(tag).chain_update(data).finalize(),
            Sha256::new()
                .chain_update(Sha256::digest(tag))
                .chain_update(Sha256::digest(tag))
                .chain_update(data)
                .finalize()
        );
    }
}
