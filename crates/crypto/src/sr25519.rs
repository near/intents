pub use schnorrkel;
use schnorrkel::{PublicKey, Signature};

use crate::Curve;

/// Sr25519 (Schnorr on Ristretto255) Digital Signature Algorithm.
///
/// Used by Polkadot / Substrate ecosystems (Polkadot.js, Talisman, Subwallet, …).
pub struct Sr25519;

impl Sr25519 {
    /// Signing context expected by `sign_simple` / `verify_simple` in
    /// `schnorrkel`. Matches what Polkadot.js Extension, Talisman, Subwallet,
    /// etc. use when signing arbitrary messages.
    pub const SIGNING_CTX: &'static [u8] = b"substrate";
}

impl Curve for Sr25519 {
    type PublicKey = PublicKey;

    type Signature = Signature;

    /// Verify Sr25519 signature over given message (of arbitrary length)
    /// for given public key.
    #[inline]
    fn verify(public_key: &Self::PublicKey, msg: &[u8], signature: &Self::Signature) -> bool {
        public_key
            .verify_simple(Self::SIGNING_CTX, msg, signature)
            .is_ok()
    }
}

#[cfg(feature = "signing")]
const _: () = {
    use core::convert::Infallible;

    use schnorrkel::Keypair;

    use crate::Signer;

    impl Signer<Sr25519> for Keypair {
        type Error = Infallible;

        #[inline]
        fn public_key(&self) -> <Sr25519 as Curve>::PublicKey {
            self.public
        }

        async fn sign(&self, msg: &[u8]) -> Result<<Sr25519 as Curve>::Signature, Self::Error> {
            Ok(self.sign_simple(Sr25519::SIGNING_CTX, msg))
        }
    }
};

/// Sr25519 public key (32-byte compressed Ristretto point).
#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(
        feature = "schemars-v0_8",
        derive(::schemars::JsonSchema),
        schemars(example = "Self::example")
    )
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::AsRef,
    derive_more::From,
    derive_more::Into,
)]
#[as_ref([u8], [u8; 32])]
#[into(owned, ref)]
#[repr(transparent)]
pub struct Sr25519PublicKey(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 32],
);

impl Sr25519PublicKey {
    #[cfg(feature = "schemars-v0_8")]
    const fn example() -> Self {
        Self(hex_literal::hex!(
            "e27d987db9ed2a7a48f4137c997d610226dc93bf256c9026268b0b8489bb9862"
        ))
    }
}

impl From<PublicKey> for Sr25519PublicKey {
    #[inline]
    fn from(value: PublicKey) -> Self {
        (&value).into()
    }
}

impl From<&PublicKey> for Sr25519PublicKey {
    #[inline]
    fn from(value: &PublicKey) -> Self {
        Self(value.to_bytes())
    }
}

impl TryFrom<Sr25519PublicKey> for PublicKey {
    type Error = schnorrkel::SignatureError;

    #[inline]
    fn try_from(value: Sr25519PublicKey) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl TryFrom<&Sr25519PublicKey> for PublicKey {
    type Error = schnorrkel::SignatureError;

    #[inline]
    fn try_from(value: &Sr25519PublicKey) -> Result<Self, Self::Error> {
        Self::from_bytes(&value.0)
    }
}

/// Sr25519 signature (64-byte Ristretto Schnorr signature).
#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(
        feature = "schemars-v0_8",
        derive(::schemars::JsonSchema),
        schemars(example = "Self::example"),
    )
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::AsRef,
    derive_more::From,
    derive_more::Into,
)]
#[as_ref([u8], [u8; 64])]
#[into(owned, ref)]
#[repr(transparent)]
pub struct Sr25519Signature(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 64],
);

impl Sr25519Signature {
    #[cfg(feature = "schemars-v0_8")]
    const fn example() -> Self {
        Self(hex_literal::hex!(
            "e2c01abbd53c89d6302475827b62c7e2168a93a407ebafd94fee3fb2e286e539"
            "ee1877c15df48c55c59f9d5e032f1f9a1b63a2dc4085517d705ec174e6c9cf8c"
        ))
    }
}

impl From<Signature> for Sr25519Signature {
    #[inline]
    fn from(value: Signature) -> Self {
        (&value).into()
    }
}

impl From<&Signature> for Sr25519Signature {
    #[inline]
    fn from(value: &Signature) -> Self {
        Self(value.to_bytes())
    }
}

impl TryFrom<Sr25519Signature> for Signature {
    type Error = schnorrkel::SignatureError;

    #[inline]
    fn try_from(value: Sr25519Signature) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl TryFrom<&Sr25519Signature> for Signature {
    type Error = schnorrkel::SignatureError;

    #[inline]
    fn try_from(value: &Sr25519Signature) -> Result<Self, Self::Error> {
        Self::from_bytes(&value.0)
    }
}

#[cfg(feature = "fmt")]
const _: () = {
    use core::{
        fmt::{self, Display},
        str::FromStr,
    };

    use crate::fmt::{ParseCurveError, TypedCurve};

    impl TypedCurve for Sr25519 {
        const CURVE_TYPE: &str = "sr25519";
    }

    impl Display for Sr25519PublicKey {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&Sr25519::to_base58(self.0))
        }
    }

    impl FromStr for Sr25519PublicKey {
        type Err = ParseCurveError;

        #[inline]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Sr25519::parse_base58(s).map(Self)
        }
    }

    impl Display for Sr25519Signature {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&Sr25519::to_base58(self.0))
        }
    }

    impl FromStr for Sr25519Signature {
        type Err = ParseCurveError;

        #[inline]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Sr25519::parse_base58(s).map(Self)
        }
    }
};

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    /// Real Polkadot.js Extension signature, kept as a regression vector.
    ///
    /// - Address: `167y8dsUr7kaM1FNoCtXWy2unEnjGHiN7ML3vawR6Nwywbci`
    /// - Message: `"<Bytes>Hello from Intents!</Bytes>"` (Polkadot.js wraps
    ///   raw messages in `<Bytes>...</Bytes>` before signing)
    #[rstest]
    #[case(
        hex!("e27d987db9ed2a7a48f4137c997d610226dc93bf256c9026268b0b8489bb9862"),
        b"<Bytes>Hello from Intents!</Bytes>".as_slice(),
        hex!(
            "e2c01abbd53c89d6302475827b62c7e2168a93a407ebafd94fee3fb2e286e539"
            "ee1877c15df48c55c59f9d5e032f1f9a1b63a2dc4085517d705ec174e6c9cf8c"
        ),
    )]
    fn verify_ok(
        #[case] public_key: impl Into<Sr25519PublicKey>,
        #[case] msg: impl AsRef<[u8]>,
        #[case] signature: impl Into<Sr25519Signature>,
    ) {
        assert!(
            Sr25519::verify(
                &public_key.into().try_into().unwrap(),
                msg.as_ref(),
                &signature.into().try_into().unwrap(),
            ),
            "signature is invalid",
        );
    }

    #[rstest]
    // Same signature, different message: MUST fail
    #[case(
        hex!("e27d987db9ed2a7a48f4137c997d610226dc93bf256c9026268b0b8489bb9862"),
        b"<Bytes>Goodbye from Intents!</Bytes>".as_slice(),
        hex!(
            "e2c01abbd53c89d6302475827b62c7e2168a93a407ebafd94fee3fb2e286e539"
            "ee1877c15df48c55c59f9d5e032f1f9a1b63a2dc4085517d705ec174e6c9cf8c"
        ),
    )]
    // Bit-flipped valid signature (first byte XOR 1): MUST fail.
    // Byte 63's top bit is left intact to keep schnorrkel's `NotMarkedSchnorrkel` marker.
    #[case(
        hex!("e27d987db9ed2a7a48f4137c997d610226dc93bf256c9026268b0b8489bb9862"),
        b"<Bytes>Hello from Intents!</Bytes>".as_slice(),
        hex!(
            "e3c01abbd53c89d6302475827b62c7e2168a93a407ebafd94fee3fb2e286e539"
            "ee1877c15df48c55c59f9d5e032f1f9a1b63a2dc4085517d705ec174e6c9cf8c"
        ),
    )]
    fn verify_fail(
        #[case] public_key: impl Into<Sr25519PublicKey>,
        #[case] msg: impl AsRef<[u8]>,
        #[case] signature: impl Into<Sr25519Signature>,
    ) {
        assert!(
            !Sr25519::verify(
                &public_key.into().try_into().unwrap(),
                msg.as_ref(),
                &signature.into().try_into().unwrap(),
            ),
            "invalid signature passed verification",
        );
    }
}
