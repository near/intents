use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use defuse_crypto::secp256k1::{Secp256k1UncompressedPublicKey, k256::ecdsa::VerifyingKey};
use defuse_digest::{Digest, sha3::Keccak256};
use serde_with::{DeserializeFromStr, SerializeDisplay};

/// [Ethereum address](https://ethereum.org/en/developers/docs/accounts/#account-creation),
/// i.e. the last 20 bytes of `keccak256(uncompressed_public_key)`, stored in
/// the [state](defuse_wallet::State) of the
/// [`WalletEip712`](crate::WalletEip712) contract variant.
///
/// # Why the address and not the public key
///
/// Ethereum wallets expose the *address* (e.g. `eth_requestAccounts`), while
/// the public key can only be recovered from a signature. Since the
/// [deterministic `AccountId`](https://github.com/near/NEPs/blob/master/neps/nep-0616.md)
/// of a wallet-contract instance is derived from its initial state, storing
/// the address means a client knows its NEAR account id right away — without
/// a preliminary signing ceremony just to learn the public key.
///
/// The contract recovers the public key from the proof and derives this
/// address from it, so the binding to the key is preserved.
#[cfg_attr(
    feature = "abi",
    derive(::borsh::BorshSchema, ::schemars::JsonSchema),
    schemars(example = "Self::example")
)]
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    SerializeDisplay,
    DeserializeFromStr,
    ::borsh::BorshSerialize,
    ::borsh::BorshDeserialize,
    derive_more::AsRef,
    derive_more::From,
    derive_more::Into,
)]
#[as_ref([u8], [u8; 20])]
#[into(owned, ref)]
#[repr(transparent)]
pub struct EthAddress(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "abi", schemars(with = "String"))] pub [u8; 20],
);

impl EthAddress {
    #[cfg(feature = "abi")]
    const fn example() -> Self {
        Self(::hex_literal::hex!(
            "d8da6bf26964af9d7eed9e03e53415d37aa96045"
        ))
    }
}

impl From<&Secp256k1UncompressedPublicKey> for EthAddress {
    #[inline]
    fn from(public_key: &Secp256k1UncompressedPublicKey) -> Self {
        Self(
            Keccak256::digest(public_key)[12..]
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
        )
    }
}

impl From<Secp256k1UncompressedPublicKey> for EthAddress {
    #[inline]
    fn from(public_key: Secp256k1UncompressedPublicKey) -> Self {
        (&public_key).into()
    }
}

impl From<&VerifyingKey> for EthAddress {
    #[inline]
    fn from(public_key: &VerifyingKey) -> Self {
        Secp256k1UncompressedPublicKey::from(public_key).into()
    }
}

impl Display for EthAddress {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl Debug for EthAddress {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl FromStr for EthAddress {
    type Err = ParseEthAddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").ok_or(ParseEthAddressError::NoPrefix)?;

        let mut address = [0u8; 20];
        // accepts both lowercase and EIP-55 checksummed (mixed-case) hex,
        // without verifying the checksum
        hex::decode_to_slice(s, &mut address)?;

        Ok(Self(address))
    }
}

/// An error returned while parsing an [`EthAddress`].
#[derive(Debug, thiserror::Error)]
pub enum ParseEthAddressError {
    #[error("missing '0x' prefix")]
    NoPrefix,

    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    /// Known-answer vector: `0x` addresses MUST be derived exactly as on
    /// Ethereum, so that the wallet is controlled by the very same key the
    /// user already has.
    #[rstest]
    #[case(
        hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"),
        "0x0551f7c9a91ee579c9e40444ffc490001c323108"
    )]
    fn derive(
        #[case] public_key: impl Into<Secp256k1UncompressedPublicKey>,
        #[case] address: &str,
    ) {
        assert_eq!(EthAddress::from(public_key.into()).to_string(), address);
    }

    /// Derivation all the way from a private key, cross-checked against an
    /// independent secp256k1 + keccak256 implementation.
    #[rstest]
    fn derive_from_private_key() {
        let key = defuse_crypto::secp256k1::k256::ecdsa::SigningKey::from_bytes(
            &hex!("4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318").into(),
        )
        .unwrap();

        assert_eq!(
            EthAddress::from(key.verifying_key()).to_string(),
            "0x2c7536e3605d9c16a7a3d7b1898e529396a65c23",
        );
    }

    #[rstest]
    #[case("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045")]
    #[case("0xd8da6bf26964af9d7eed9e03e53415d37aa96045")]
    fn parse(#[case] s: &str) {
        assert_eq!(
            EthAddress::from_str(s).unwrap(),
            EthAddress(hex!("d8da6bf26964af9d7eed9e03e53415d37aa96045")),
        );
    }

    #[rstest]
    #[case::no_prefix("d8da6bf26964af9d7eed9e03e53415d37aa96045")]
    #[case::too_short("0xd8da6bf26964af9d7eed9e03e53415d37aa960")]
    #[case::not_hex("0xzzda6bf26964af9d7eed9e03e53415d37aa96045")]
    fn parse_fail(#[case] s: &str) {
        assert!(EthAddress::from_str(s).is_err());
    }
}
