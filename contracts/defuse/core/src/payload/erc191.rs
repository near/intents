use super::{DefusePayload, ExtractDefusePayload, Payload, SignedPayload};
use defuse_crypto::{Curve, CurveTypes, Secp256k1};
use defuse_erc191::{Erc191Payload, SignedErc191Payload};
use near_sdk::{serde::de::DeserializeOwned, serde_json};
use defuse_digest::{Sha256, Digest};

impl Payload for Erc191Payload {
    #[inline]
    fn hash(&self) -> defuse_crypto::CryptoHash {
        defuse_digest::Sha256::digest(self.prehash()).into()
    }
}

impl Payload for SignedErc191Payload {
    #[inline]
    fn hash(&self) -> defuse_crypto::CryptoHash {
        self.payload.hash()
    }
}

impl SignedPayload for SignedErc191Payload {
    type PublicKey = <Secp256k1 as CurveTypes>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        <Secp256k1 as Curve>::verify(&self.signature, &Payload::hash(&self.payload), &())
    }
}
impl<T> ExtractDefusePayload<T> for SignedErc191Payload
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    #[inline]
    fn extract_defuse_payload(self) -> Result<DefusePayload<T>, Self::Error> {
        serde_json::from_str(&self.payload.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    const fn fix_v_in_signature(mut sig: [u8; 65]) -> [u8; 65] {
        if *sig.last().unwrap() >= 27 {
            *sig.last_mut().unwrap() -= 27;
        }
        sig
    }

    // Signature constructed in Metamask, using private key: a4b319a82adfc43584e4537fec97a80516e16673db382cd91eba97abbab8ca56
    const REFERENCE_SIGNATURE: [u8; 65] = hex!(
        "7800a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e471c"
    );
    const INVALID_REFERENCE_SIGNATURE: [u8; 65] = hex!(
        "7900a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e471c"
    );
    const REFERENCE_MESSAGE: &str = "Hello world!";
    const INVALID_REFERENCE_MESSAGE: &str = "Hello, NEAR!";
    const REFERENCE_PUBKEY: [u8; 64] = hex!(
        "85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"
    );

    #[test]
    fn test_reference_signature_verification_works() {
        assert_eq!(
            SignedErc191Payload {
                payload: Erc191Payload(REFERENCE_MESSAGE.to_string()),
                signature: fix_v_in_signature(REFERENCE_SIGNATURE),
            }
            .verify(),
            Some(REFERENCE_PUBKEY)
        );
    }

    #[test]
    fn test_invalid_reference_message_verification_fails() {
        assert_ne!(
            SignedErc191Payload {
                payload: Erc191Payload(INVALID_REFERENCE_MESSAGE.to_string()),
                signature: fix_v_in_signature(REFERENCE_SIGNATURE),
            }
            .verify(),
            Some(REFERENCE_PUBKEY)
        );
    }

    #[test]
    fn test_invalid_reference_signature_verification_fails() {
        assert_ne!(
            SignedErc191Payload {
                payload: Erc191Payload(REFERENCE_MESSAGE.to_string()),
                signature: fix_v_in_signature(INVALID_REFERENCE_SIGNATURE),
            }
            .verify(),
            Some(REFERENCE_PUBKEY)
        );
    }
}
