//! Helper crate to automatically chose a backend for digest implementations.
//! Currently supported backends are:
//!
//! * `cfg(near)`: via Near host-function
//! * default: fallback to pure Rust implementation

pub use digest::*;

cfg_select! {
    near => {
        mod near;
        pub use self::near::*;
    }
    _ => {
        #[cfg(feature = "sha2")]
        pub use sha2;
        #[cfg(feature = "sha3")]
        pub use sha3;
    }
}

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;

    use digest::{Digest, array::ArraySize, common::OutputSize};
    use hex_literal::hex;
    use rstest::rstest;

    // XXX: `near-sdk` was added in order to enable tests and doctests compiling with mockchain
    #[cfg(near)]
    use near_sdk as _;

    use super::*;

    #[rstest]
    #[case(
        PhantomData::<sha2::Sha256>,
        b"",
        hex!("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
    )]
    #[case(
        PhantomData::<sha2::Sha256>,
        b"test",
        hex!("9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"),
    )]
    #[case(
        PhantomData::<sha3::Keccak256>,
        b"",
        hex!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"),
    )]
    #[case(
        PhantomData::<sha3::Keccak256>,
        b"test",
        hex!("9c22ff5f21f0b81b113e63f7db6da94fedef11b2119b4088b89664fb9a3cb658"),
    )]
    #[case(
        PhantomData::<sha3::Keccak512>,
        b"",
        hex!("0eab42de4c3ceb9235fc91acffe746b29c29a8c366b7c60e4e67c466f36a4304c00fa9caf9d87976ba469bcbe06713b435f091ef2769fb160cdab33d3670680e"),
    )]
    #[case(
        PhantomData::<sha3::Keccak512>,
        b"test",
        hex!("1e2e9fc2002b002d75198b7503210c05a1baac4560916a3c6d93bcce3a50d7f00fd395bf1647b9abb8d1afcc9c76c289b0c9383ba386a956da4b38934417789e"),
    )]
    #[allow(clippy::used_underscore_binding)]
    fn has_not_changed<D>(
        #[case] _d: PhantomData<D>,
        #[case] data: &[u8],
        #[case] output: <OutputSize<D> as ArraySize>::ArrayType<u8>,
    ) where
        D: Digest,
        <OutputSize<D> as ArraySize>::ArrayType<u8>: PartialEq,
    {
        assert!(D::digest(data).0 == output, "has changed");
    }
}
