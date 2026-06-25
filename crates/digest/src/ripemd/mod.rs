use crate::digest_cfg;

#[cfg(near)]
mod near;

digest_cfg! {
    pub struct Ripemd160 {
        near => self::near::Ripemd160,
        _ => ::ripemd::Ripemd160,
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
        hex!("9c1185a5c5e9fc54612808977ee8f548b2258d31"),
    )]
    #[case(
        b"test",
        hex!("5e52fee47e6b070565f74372468cdc699de89107"),
    )]
    fn ripemd160_has_not_changed(#[case] data: &[u8], #[case] output: [u8; 20]) {
        assert!(Ripemd160::digest(data) == output, "has changed");
    }
}
