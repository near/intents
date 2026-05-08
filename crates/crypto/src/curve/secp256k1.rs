use crate::TypedCurve;
use core::fmt::{self, Debug, Display};
use std::str::FromStr;

use crate::{CryptoHash, CurveTypes, ParseCurveError};

pub struct Secp256k1;

impl CurveTypes for Secp256k1 {
    type PublicKey = [u8; 64];

    /// Concatenated `r`, `s` and `v` (recovery byte).
    ///
    /// Note: Ethereum clients shift the recovery byte and this
    /// logic might depend on chain id, so clients must rollback
    /// these changes to v ∈ {0, 1}.
    /// References:
    /// * <https://github.com/ethereumjs/ethereumjs-monorepo/blob/dc7169c16df6d36adeb6e234fcc66eb6cfc5ea3f/packages/util/src/signature.ts#L31-L62>
    /// * <https://github.com/ethereum/go-ethereum/issues/19751#issuecomment-504900739>
    type Signature = [u8; 65];

    // Output of cryptographic hash function
    type Message = CryptoHash;

    /// ECDSA signatures are recoverable, so you don't need a verifying key
    type VerifyingKey = ();
}

#[cfg(feature = "near-contract")]
impl crate::Curve for Secp256k1 {
    #[inline]
    fn verify(
        [signature @ .., v]: &Self::Signature,
        hash: &Self::Message,
        _verifying_key: &(),
    ) -> Option<Self::PublicKey> {
        near_sdk::env::ecrecover(
            hash, signature, *v,
            // Do not accept malleable signatures:
            // https://github.com/near/nearcore/blob/d73041cc1d1a70af4456fceefaceb1bf7f684fde/core/crypto/src/signature.rs#L448-L455
            true,
        )
    }
}

impl TypedCurve for Secp256k1 {
    const CURVE_TYPE: crate::CurveType = crate::CurveType::Secp256k1;
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr)
)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Secp256k1PublicKey(pub <Secp256k1 as CurveTypes>::PublicKey);

impl Debug for Secp256k1PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Secp256k1PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&<Secp256k1 as TypedCurve>::to_base58(self.0))
    }
}

impl FromStr for Secp256k1PublicKey {
    type Err = ParseCurveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Secp256k1::parse_base58(s).map(Self)
    }
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr)
)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Secp256k1Signature(pub <Secp256k1 as CurveTypes>::Signature);

impl Debug for Secp256k1Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Secp256k1Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&<Secp256k1 as TypedCurve>::to_base58(self.0))
    }
}

impl FromStr for Secp256k1Signature {
    type Err = ParseCurveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Secp256k1::parse_base58(s).map(Self)
    }
}
