pub use k256;

use k256::{
    EncodedPoint,
    ecdsa::{RecoveryId, Signature, VerifyingKey},
};

use crate::{Curve, RecoverableCurve};

pub struct Secp256k1;

// TODO: docs
/// Prehash, i.e. output of a cryptographic hash function
impl Curve for Secp256k1 {
    type PublicKey = VerifyingKey;
    type Signature = Signature;

    // TODO: docs: prehash
    #[inline]
    fn verify(public_key: &VerifyingKey, prehash: &[u8], signature: &Self::Signature) -> bool {
        // accept only 32 byte prehash
        let Ok(prehash) = <&[u8; 32]>::try_from(prehash) else {
            return false;
        };

        cfg_select! {
            near => {
                // `near_sdk::env::ecrecover` requires recovery_id, so
                // we need to find one trial recovery
                for id in 0..=RecoveryId::MAX {
                    let recovery_id = RecoveryId::from_byte(id).unwrap_or_else(|| unreachable!());

                    if let Some(recovered) = Self::recover(prehash, signature, recovery_id)
                        && recovered == *public_key
                    {
                        return true;
                    }
                }
                // no recovery id was found
                false
            }
            _ => {{
                use k256::{
                    ecdsa::signature::hazmat::PrehashVerifier,
                    elliptic_curve::scalar::IsHigh,
                };

                if signature.s().is_high().into() {
                    // guard against signature malleability
                    return false;
                }

                // TODO: other checks?
                public_key.verify_prehash(prehash, signature).is_ok()
            }}
        }
    }
}

impl RecoverableCurve for Secp256k1 {
    type RecoveryId = RecoveryId;

    #[inline]
    fn recover(
        prehash: &[u8],
        signature: &Self::Signature,
        recovery_id: Self::RecoveryId,
    ) -> Option<Self::PublicKey> {
        // accept only 32 byte prehash
        let prehash = <&[u8; 32]>::try_from(prehash).ok()?;

        let public_key = {
            cfg_select! {
                near => {
                    use k256::EncodedPoint;

                    let pk: [u8; 64] = ::near_sdk::env::ecrecover(
                        prehash,
                        &signature.to_bytes(),
                        recovery_id.to_byte(),
                        // Do not accept malleable signatures:
                        // https://github.com/near/nearcore/blob/d73041cc1d1a70af4456fceefaceb1bf7f684fde/core/crypto/src/signature.rs#L448-L455
                        true,
                    )?;

                    VerifyingKey::from_encoded_point(&EncodedPoint::from_untagged_bytes(&pk.into())).ok()
                }
                _ => {
                    use k256::elliptic_curve::scalar::IsHigh;

                    if signature.s().is_high().into() {
                        // guard against signature malleability
                        return None;
                    }

                    VerifyingKey::recover_from_prehash(prehash, signature, recovery_id).ok()
                }
            }
        }?;

        debug_assert!(
            Self::verify(&public_key, prehash, signature),
            "invalid recovered public key",
        );

        Some(public_key)
    }
}

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
// TODO: docs: untagged uncompressed with no leading SEC-1 tag byte, etc...
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Secp256k1UncompressedPublicKey(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 64],
);

impl From<VerifyingKey> for Secp256k1UncompressedPublicKey {
    #[inline]
    fn from(value: VerifyingKey) -> Self {
        (&value).into()
    }
}

impl From<&VerifyingKey> for Secp256k1UncompressedPublicKey {
    #[inline]
    fn from(value: &VerifyingKey) -> Self {
        Self(
            value
                .to_encoded_point(false) // do not compress
                // TODO: this might fail for identity (and maybe other cases?)
                .as_bytes()[1..] // skip SEC-1 leading tag byte
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
        )
    }
}

impl TryFrom<Secp256k1UncompressedPublicKey> for VerifyingKey {
    type Error = k256::ecdsa::Error;

    #[inline]
    fn try_from(value: Secp256k1UncompressedPublicKey) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl TryFrom<&Secp256k1UncompressedPublicKey> for VerifyingKey {
    type Error = k256::ecdsa::Error;

    #[inline]
    fn try_from(value: &Secp256k1UncompressedPublicKey) -> Result<Self, Self::Error> {
        Self::from_encoded_point(&EncodedPoint::from_untagged_bytes((&value.0).into()))
    }
}

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
// TODO: docs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Secp256k1RecoverableSignature(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 65],
);

impl From<(Signature, RecoveryId)> for Secp256k1RecoverableSignature {
    #[inline]
    fn from((signature, recovery_id): (Signature, RecoveryId)) -> Self {
        (&signature, recovery_id).into()
    }
}

impl From<(&Signature, RecoveryId)> for Secp256k1RecoverableSignature {
    #[inline]
    fn from((signature, recovery_id): (&Signature, RecoveryId)) -> Self {
        let mut buf = [0u8; 65];
        buf[..64].copy_from_slice(&signature.to_bytes());
        buf[64] = recovery_id.to_byte();
        Self(buf)
    }
}

impl TryFrom<Secp256k1RecoverableSignature> for (Signature, RecoveryId) {
    type Error = k256::ecdsa::Error;

    #[inline]
    fn try_from(value: Secp256k1RecoverableSignature) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl TryFrom<&Secp256k1RecoverableSignature> for (Signature, RecoveryId) {
    type Error = k256::ecdsa::Error;

    #[inline]
    fn try_from(value: &Secp256k1RecoverableSignature) -> Result<Self, Self::Error> {
        let [ref signature @ .., recovery_id] = value.0;
        Ok((
            Signature::from_bytes(signature.into())?,
            RecoveryId::from_byte(recovery_id).ok_or_else(k256::ecdsa::Error::new)?,
        ))
    }
}

#[cfg(feature = "fmt")]
const _: () = {
    use core::{
        fmt::{self, Display},
        str::FromStr,
    };

    use crate::fmt::{ParseCurveError, TypedCurve};

    impl TypedCurve for Secp256k1 {
        const CURVE_TYPE: &str = "secp256k1";
    }

    impl Display for Secp256k1UncompressedPublicKey {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&Secp256k1::to_base58(&self.0))
        }
    }

    impl FromStr for Secp256k1UncompressedPublicKey {
        type Err = ParseCurveError;

        #[inline]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Secp256k1::parse_base58(s).map(Self)
        }
    }

    impl Display for Secp256k1RecoverableSignature {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&Secp256k1::to_base58(&self.0))
        }
    }

    impl FromStr for Secp256k1RecoverableSignature {
        type Err = ParseCurveError;

        #[inline]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Secp256k1::parse_base58(s).map(Self)
        }
    }
};

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use k256::EncodedPoint;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"),
        hex!("aa05af77f274774b8bdc7b61d98bc40da523dc2821fdea555f4d6aa413199bcc"),
        hex!("7800a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e47"),
    )]
    #[case(
        hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"),
        hex!("1632c0ebba467e157675403ba3ba280b836e1801b5678d878dfc90bfc403d6e1"),
        hex!("eea1651a60600ec4d9c45e8ae81da1a78377f789f0ac2019de66ad943459913015ef9256809ee0e6bb76e303a0b4802e475c1d26ade5d585292b80c9fe9cb10c"),
    )]
    fn verify_ok(
        #[case] public_key: [u8; 64],
        #[case] prehash: [u8; 32],
        #[case] signature: [u8; 64],
    ) {
        let public_key = VerifyingKey::from_encoded_point(&EncodedPoint::from_untagged_bytes(
            &public_key.into(),
        ))
        .unwrap();
        let signature = Signature::from_bytes(&signature.into()).unwrap();

        assert!(Secp256k1::verify(&public_key, &prehash, &signature));
    }

    #[rstest]
    #[case(
        hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"),
        hex!("1632c0ebba467e157675403ba3ba280b836e1801b5678d878dfc90bfc403d6e1"),
        hex!("7800a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e47"),
    )]
    #[case(
        hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"),
        hex!("aa05af77f274774b8bdc7b61d98bc40da523dc2821fdea555f4d6aa413199bcc"),
        hex!("eea1651a60600ec4d9c45e8ae81da1a78377f789f0ac2019de66ad943459913015ef9256809ee0e6bb76e303a0b4802e475c1d26ade5d585292b80c9fe9cb10c"),
    )]
    fn verify_fail(
        #[case] public_key: [u8; 64],
        #[case] prehash: [u8; 32],
        #[case] signature: [u8; 64],
    ) {
        let public_key = VerifyingKey::from_encoded_point(&EncodedPoint::from_untagged_bytes(
            &public_key.into(),
        ))
        .unwrap();
        let signature = Signature::from_bytes(&signature.into()).unwrap();

        assert!(!Secp256k1::verify(&public_key, &prehash, &signature));
    }

    #[rstest]
    #[case(
        hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"),
        hex!("aa05af77f274774b8bdc7b61d98bc40da523dc2821fdea555f4d6aa413199bcc"),
        hex!("7800a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e4701"),
    )]
    #[case(
        hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"),
        hex!("1632c0ebba467e157675403ba3ba280b836e1801b5678d878dfc90bfc403d6e1"),
        hex!("eea1651a60600ec4d9c45e8ae81da1a78377f789f0ac2019de66ad943459913015ef9256809ee0e6bb76e303a0b4802e475c1d26ade5d585292b80c9fe9cb10c01"),
    )]
    fn recover_ok(
        #[case] public_key: [u8; 64],
        #[case] prehash: [u8; 32],
        #[case] signature: [u8; 65],
    ) {
        let public_key = VerifyingKey::from_encoded_point(&EncodedPoint::from_untagged_bytes(
            &public_key.into(),
        ))
        .unwrap();
        let [signature @ .., v] = signature;
        let signature = Signature::from_bytes(&signature.into()).unwrap();
        let recovery_id = RecoveryId::from_byte(v).unwrap_or_else(|| unreachable!());

        assert_eq!(
            Secp256k1::recover(&prehash, &signature, recovery_id),
            Some(public_key)
        );
    }
}
