use defuse_crypto::{Curve, Secp256k1};
#[cfg(feature = "serde")]
use defuse_crypto::serde::AsCurve;
#[cfg(feature = "near-contract")]
use defuse_crypto::{CryptoHash, Payload, SignedPayload};
use impl_tools::autoimpl;

/// See [ERC-191](https://github.com/ethereum/ercs/blob/master/ERCS/erc-191.md)
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone)]
pub struct Erc191Payload(pub String);

impl Erc191Payload {
    #[inline]
    pub fn prehash(&self) -> Vec<u8> {
        let data = self.0.as_bytes();
        [
            format!("\x19Ethereum Signed Message:\n{}", data.len()).as_bytes(),
            data,
        ]
        .concat()
    }
}

#[cfg(feature = "near-contract")]
impl Payload for Erc191Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        near_sdk::env::keccak256_array(self.prehash())
    }
}

#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[autoimpl(Deref using self.payload)]
#[derive(Debug, Clone)]
pub struct SignedErc191Payload {
    pub payload: Erc191Payload,

    /// There is no public key member because the public key can be recovered
    /// via `ecrecover()` knowing the data and the signature
    #[cfg_attr(feature = "serde", serde_as(as = "AsCurve<Secp256k1>"))]
    pub signature: <Secp256k1 as Curve>::Signature,
}

#[cfg(feature = "near-contract")]
impl Payload for SignedErc191Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        self.payload.hash()
    }
}

#[cfg(feature = "near-contract")]
impl SignedPayload for SignedErc191Payload {
    type PublicKey = <Secp256k1 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Secp256k1::verify(&self.signature, &self.payload.hash(), &())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use defuse_crypto::Secp256k1;
    use hex_literal::hex;
    use sha3::Digest as _;

    const fn fix_v_in_signature(mut sig: [u8; 65]) -> [u8; 65] {
        if *sig.last().unwrap() >= 27 {
            // Ethereum only uses uncompressed keys, with corresponding value v=27/28
            // https://bitcoin.stackexchange.com/a/38909/58790
            *sig.last_mut().unwrap() -= 27;
        }
        sig
    }

    fn hash(payload: &Erc191Payload) -> [u8; 32] {
        sha3::Keccak256::digest(payload.prehash()).into()
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

    // Public key can be derived using `ethers_signers` crate:
    // let wallet = LocalWallet::from_str(
    //     "a4b319a82adfc43584e4537fec97a80516e16673db382cd91eba97abbab8ca56",
    // )?;
    // let signing_key = wallet.signer();
    // let verifying_key = signing_key.verifying_key();
    // let public_key = verifying_key.to_encoded_point(false);
    // // Notice that we skip the first byte, 0x04
    // println!("Public key: 0x{}", hex::encode(public_key.as_bytes()[1..]));
    const REFERENCE_PUBKEY: [u8; 64] = hex!(
        "85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"
    );

    #[test]
    fn test_reference_signature_verification_works() {
        let payload = Erc191Payload(REFERENCE_MESSAGE.to_string());
        assert_eq!(
            Secp256k1::verify(&fix_v_in_signature(REFERENCE_SIGNATURE), &hash(&payload), &()),
            Some(REFERENCE_PUBKEY)
        );
    }

    #[test]
    fn test_invalid_reference_message_verification_fails() {
        let payload = Erc191Payload(INVALID_REFERENCE_MESSAGE.to_string());
        assert_ne!(
            Secp256k1::verify(&fix_v_in_signature(REFERENCE_SIGNATURE), &hash(&payload), &()),
            Some(REFERENCE_PUBKEY)
        );
    }

    #[test]
    fn test_invalid_reference_signature_verification_fails() {
        let payload = Erc191Payload(REFERENCE_MESSAGE.to_string());
        assert_ne!(
            Secp256k1::verify(&fix_v_in_signature(INVALID_REFERENCE_SIGNATURE), &hash(&payload), &()),
            Some(REFERENCE_PUBKEY)
        );
    }
}
