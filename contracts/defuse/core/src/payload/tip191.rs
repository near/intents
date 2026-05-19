use super::{DefusePayload, ExtractDefusePayload, Payload, SignedPayload};
use defuse_crypto::{Curve, Secp256k1, VerifiableCurve};
use defuse_digest::{Digest, Keccak256};
use defuse_tip191::{SignedTip191Payload, Tip191Payload};
use near_sdk::{serde::de::DeserializeOwned, serde_json};

impl Payload for Tip191Payload {
    #[inline]
    fn hash(&self) -> defuse_crypto::CryptoHash {
        Keccak256::digest(self.prehash()).into()
    }
}

impl Payload for SignedTip191Payload {
    #[inline]
    fn hash(&self) -> defuse_crypto::CryptoHash {
        self.payload.hash()
    }
}

impl SignedPayload for SignedTip191Payload {
    type PublicKey = <Secp256k1 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Secp256k1::verify(&self.signature, &self.payload.hash(), &())
    }
}

impl<T> ExtractDefusePayload<T> for SignedTip191Payload
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
            // Ethereum only uses uncompressed keys, with corresponding value v=27/28
            // https://bitcoin.stackexchange.com/a/38909/58790
            *sig.last_mut().unwrap() -= 27;
        }
        sig
    }

    // NOTE: Public key can be derived using `ethers_signers` crate:
    // let wallet = LocalWallet::from_str(
    //     "a4b319a82adfc43584e4537fec97a80516e16673db382cd91eba97abbab8ca56",
    // )?;
    // let signing_key = wallet.signer();
    // let verifying_key = signing_key.verifying_key();
    // let public_key = verifying_key.to_encoded_point(false);
    // // Notice that we skip the first byte, 0x04
    // println!("Public key: 0x{}", hex::encode(public_key.as_bytes()[1..]));

    const REFERENCE_MESSAGE: &str = "Hello, TRON!";
    const INVALID_REFERENCE_MESSAGE: &str = "this is not TRON reference input message";
    const REFERENCE_SIGNATURE: [u8; 65] = hex!(
        "eea1651a60600ec4d9c45e8ae81da1a78377f789f0ac2019de66ad943459913015ef9256809ee0e6bb76e303a0b4802e475c1d26ade5d585292b80c9fe9cb10c1c"
    );
    const INVALID_REFERENCE_SIGNATURE: [u8; 65] = hex!(
        "0000000011111111000000001110111110000000011111111e66ad943459913015ef9256809ee0e6bb76e303a0b4802e475c1d26ade5d585292b80c9fe9cb10c1c"
    );
    const REFERENCE_PUBKEY: [u8; 64] = hex!(
        "85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"
    );

    #[test]
    fn test_reference_signature_verification_works() {
        assert_eq!(
            SignedTip191Payload {
                payload: Tip191Payload(REFERENCE_MESSAGE.to_string()),
                signature: fix_v_in_signature(REFERENCE_SIGNATURE),
            }
            .verify(),
            Some(REFERENCE_PUBKEY)
        );
    }

    #[test]
    fn test_invalid_reference_message_verification_fails() {
        assert_ne!(
            SignedTip191Payload {
                payload: Tip191Payload(INVALID_REFERENCE_MESSAGE.to_string()),
                signature: fix_v_in_signature(REFERENCE_SIGNATURE),
            }
            .verify(),
            Some(REFERENCE_PUBKEY)
        );
    }

    #[test]
    fn test_invalid_reference_signature_verification_fails() {
        assert_ne!(
            SignedTip191Payload {
                payload: Tip191Payload(REFERENCE_MESSAGE.to_string()),
                signature: fix_v_in_signature(INVALID_REFERENCE_SIGNATURE),
            }
            .verify(),
            Some(REFERENCE_PUBKEY)
        );
    }
}
