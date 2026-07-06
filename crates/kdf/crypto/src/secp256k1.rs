use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};

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
        cfg_select! {
            near => {
                use k256::EncodedPoint;

                let sig: [u8; 64] = signature.to_bytes().into();
                let pk: [u8; 64] = ::near_sdk::env::ecrecover(
                    &prehash,
                    &sig,
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

                VerifyingKey::recover_from_prehash(&prehash, signature, recovery_id).ok()
            }
        }
    }
}

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
