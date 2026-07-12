pub use ed25519_dalek;
use ed25519_dalek::{Signature, VerifyingKey};

use crate::Curve;

/// Ed25519 Digital Signature Algorithm
pub struct Ed25519;

impl Curve for Ed25519 {
    type PublicKey = VerifyingKey;

    type Signature = Signature;

    /// Verify ed25519 signature over given message (of arbitrary length)
    /// for given public key. Hashing the
    #[inline]
    fn verify(public_key: &Self::PublicKey, msg: &[u8], signature: &Self::Signature) -> bool {
        if public_key.is_weak() {
            // prevent using weak (i.e. low order) public keys, see
            // https://github.com/dalek-cryptography/ed25519-dalek#weak-key-forgery-and-verify_strict
            return false;
        }

        cfg_select! {
            near => {
                ::near_sdk::env::ed25519_verify(
                    &signature.to_bytes(),
                    msg,
                    public_key.as_bytes(),
                )
            }
            _ => {{
                use ed25519_dalek::Verifier;

                public_key.verify(msg, signature).is_ok()
            }}
        }
    }
}

/// Ed25519 public key
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
pub struct Ed25519PublicKey(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 32],
);

impl Ed25519PublicKey {
    #[cfg(feature = "schemars-v0_8")]
    const fn example() -> Self {
        Self(hex_literal::hex!(
            "8565df94b8caab08f28cdd2ee014b800915741d4694fa840e50cca02ae5c6466"
        ))
    }
}

impl From<VerifyingKey> for Ed25519PublicKey {
    #[inline]
    fn from(value: VerifyingKey) -> Self {
        (&value).into()
    }
}

impl From<&VerifyingKey> for Ed25519PublicKey {
    #[inline]
    fn from(value: &VerifyingKey) -> Self {
        Self(value.to_bytes())
    }
}

impl TryFrom<Ed25519PublicKey> for VerifyingKey {
    type Error = ed25519_dalek::ed25519::Error;

    #[inline]
    fn try_from(value: Ed25519PublicKey) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl TryFrom<&Ed25519PublicKey> for VerifyingKey {
    type Error = ed25519_dalek::ed25519::Error;

    #[inline]
    fn try_from(value: &Ed25519PublicKey) -> Result<Self, Self::Error> {
        Self::from_bytes(&value.0)
    }
}

/// Ed25519 signature
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
pub struct Ed25519Signature(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 64],
);

impl Ed25519Signature {
    #[cfg(feature = "schemars-v0_8")]
    const fn example() -> Self {
        Self(hex_literal::hex!(
            "e4822e15e5988bf08c80b72f2d1292b7229029f342d42bb9dfe4e230c66c10a6c4a86a47ddc58b1446baedf2f1312294d59638c812082a0124e513d4eb16c40e"
        ))
    }
}

impl From<Signature> for Ed25519Signature {
    #[inline]
    fn from(value: Signature) -> Self {
        (&value).into()
    }
}

impl From<&Signature> for Ed25519Signature {
    #[inline]
    fn from(value: &Signature) -> Self {
        Self(value.to_bytes())
    }
}

impl From<Ed25519Signature> for Signature {
    #[inline]
    fn from(value: Ed25519Signature) -> Self {
        (&value).into()
    }
}

impl From<&Ed25519Signature> for Signature {
    #[inline]
    fn from(value: &Ed25519Signature) -> Self {
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

    impl TypedCurve for Ed25519 {
        const CURVE_TYPE: &str = "ed25519";
    }

    impl Display for Ed25519PublicKey {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&Ed25519::to_base58(self.0))
        }
    }

    impl FromStr for Ed25519PublicKey {
        type Err = ParseCurveError;

        #[inline]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ed25519::parse_base58(s).map(Self)
        }
    }

    impl Display for Ed25519Signature {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&Ed25519::to_base58(self.0))
        }
    }

    impl FromStr for Ed25519Signature {
        type Err = ParseCurveError;

        #[inline]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ed25519::parse_base58(s).map(Self)
        }
    }
};

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        hex!("8565df94b8caab08f28cdd2ee014b800915741d4694fa840e50cca02ae5c6466"),
        hex!("060fab6e0fa2ea8913ef80f6f4c6fd8c0c24c2ac044d73b837d887b1e6f378fa"),
        hex!("e4822e15e5988bf08c80b72f2d1292b7229029f342d42bb9dfe4e230c66c10a6c4a86a47ddc58b1446baedf2f1312294d59638c812082a0124e513d4eb16c40e"),
    )]
    #[case(
        hex!("5231b8bba197e888c447ff6617d33dbb7fa571cdbbfb93f0b845c2293c86a3f0"),
        hex!("15401eb21a14a1f9b21277cd65e4e985e094a465c2939c13b39a56b4043a2cdc"),
        hex!("024068f38742fa99be08b9779745562ba10ce336de5865497fc5442353e355d90c1eb986d04fb70d1031b0a7f7cfe80946e0cd3979316b522fbdb8ed35028f0f"),
    )]
    fn verify_ok(
        #[case] public_key: impl Into<Ed25519PublicKey>,
        #[case] msg: impl AsRef<[u8]>,
        #[case] signature: impl Into<Ed25519Signature>,
    ) {
        assert!(
            Ed25519::verify(
                &public_key.into().try_into().unwrap(),
                msg.as_ref(),
                &signature.into().into(),
            ),
            "signature is invalid",
        );
    }

    #[rstest]
    #[case(
        hex!("8565df94b8caab08f28cdd2ee014b800915741d4694fa840e50cca02ae5c6466"),
        hex!("94fde20581344b29a34224eadd55ceff65afc94148f550255d36e1de9ec064d0"),
        hex!("e4822e15e5988bf08c80b72f2d1292b7229029f342d42bb9dfe4e230c66c10a6c4a86a47ddc58b1446baedf2f1312294d59638c812082a0124e513d4eb16c40e"),
    )]
    #[case(
        hex!("5231b8bba197e888c447ff6617d33dbb7fa571cdbbfb93f0b845c2293c86a3f0"),
        hex!("060fab6e0fa2ea8913ef80f6f4c6fd8c0c24c2ac044d73b837d887b1e6f378fa"),
        hex!("e4822e15e5988bf08c80b72f2d1292b7229029f342d42bb9dfe4e230c66c10a6c4a86a47ddc58b1446baedf2f1312294d59638c812082a0124e513d4eb16c40e"),
    )]
    #[case(
        hex!("8565df94b8caab08f28cdd2ee014b800915741d4694fa840e50cca02ae5c6466"),
        hex!("060fab6e0fa2ea8913ef80f6f4c6fd8c0c24c2ac044d73b837d887b1e6f378fa"),
        hex!("024068f38742fa99be08b9779745562ba10ce336de5865497fc5442353e355d90c1eb986d04fb70d1031b0a7f7cfe80946e0cd3979316b522fbdb8ed35028f0f"),
    )]
    fn verify_fail(
        #[case] public_key: impl Into<Ed25519PublicKey>,
        #[case] msg: impl AsRef<[u8]>,
        #[case] signature: impl Into<Ed25519Signature>,
    ) {
        assert!(
            !Ed25519::verify(
                &public_key.into().try_into().unwrap(),
                msg.as_ref(),
                &signature.into().into(),
            ),
            "invalid signature passed verification",
        );
    }
}
