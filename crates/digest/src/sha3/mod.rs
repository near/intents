use crate::digest_cfg;

#[cfg(near)]
mod near;

digest_cfg! {
    pub struct Keccak256 {
        near => crate::utils::DigestFn::<self::near::Keccak256Fn>,
        _ => ::sha3::Keccak256,
    }
}

digest_cfg! {
    pub struct Keccak512 {
        near => crate::utils::DigestFn::<self::near::Keccak512Fn>,
        _ => ::sha3::Keccak512,
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
}
