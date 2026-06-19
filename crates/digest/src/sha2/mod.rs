use crate::digest_cfg;

#[cfg(near)]
mod near;

digest_cfg! {
    pub struct Sha256 {
        near => self::near::Sha256,
        _ => ::sha2::Sha256,
    }
}

#[cfg(test)]
mod tests {
    use digest::Digest;
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        b"",
        hex!("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
    )]
    #[case(
        b"test",
        hex!("9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"),
    )]
    fn sha256_has_not_changed(#[case] data: &[u8], #[case] output: [u8; 32]) {
        assert!(Sha256::digest(data) == output, "has changed");
    }
}
