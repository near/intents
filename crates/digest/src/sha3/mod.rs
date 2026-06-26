use crate::digest_cfg;

#[cfg(near)]
mod near;

digest_cfg! {
    pub struct Keccak256 {
        near => self::near::Keccak256,
        _ => ::sha3::Keccak256,
    }
}

digest_cfg! {
    pub struct Keccak512 {
        near => self::near::Keccak512,
        _ => ::sha3::Keccak512,
    }
}

digest_cfg! {
    pub struct Sha3_256 {
        // TODO: cfg(near)
        _ => ::sha3::Sha3_256,
    }
}

digest_cfg! {
    pub struct Sha3_512 {
        // TODO: cfg(near)
        _ => ::sha3::Sha3_512,
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
        hex!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"),
    )]
    #[case(
        b"test",
        hex!("9c22ff5f21f0b81b113e63f7db6da94fedef11b2119b4088b89664fb9a3cb658"),
    )]
    fn keccak256_has_not_changed(#[case] data: &[u8], #[case] output: [u8; 32]) {
        assert!(Keccak256::digest(data) == output, "has changed");
    }

    #[rstest]
    #[case(
        b"",
        hex!("0eab42de4c3ceb9235fc91acffe746b29c29a8c366b7c60e4e67c466f36a4304c00fa9caf9d87976ba469bcbe06713b435f091ef2769fb160cdab33d3670680e"),
    )]
    #[case(
        b"test",
        hex!("1e2e9fc2002b002d75198b7503210c05a1baac4560916a3c6d93bcce3a50d7f00fd395bf1647b9abb8d1afcc9c76c289b0c9383ba386a956da4b38934417789e"),
    )]
    fn keccak512_has_not_changed(#[case] data: &[u8], #[case] output: [u8; 64]) {
        assert!(Keccak512::digest(data) == output, "has changed");
    }

    #[rstest]
    #[case(
        b"",
        hex!("a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"),
    )]
    #[case(
        b"test",
        hex!("36f028580bb02cc8272a9a020f4200e346e276ae664e45ee80745574e2f5ab80"),
    )]
    fn sha3_256_has_not_changed(#[case] data: &[u8], #[case] output: [u8; 32]) {
        assert!(Sha3_256::digest(data) == output, "has changed");
    }

    #[rstest]
    #[case(
        b"",
        hex!("a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26"),
    )]
    #[case(
        b"test",
        hex!("9ece086e9bac491fac5c1d1046ca11d737b92a2b2ebd93f005d7b710110c0a678288166e7fbe796883a4f2e9b3ca9f484f521d0ce464345cc1aec96779149c14"),
    )]
    fn sha3_512_has_not_changed(#[case] data: &[u8], #[case] output: [u8; 64]) {
        assert!(Sha3_512::digest(data) == output, "has changed");
    }
}
