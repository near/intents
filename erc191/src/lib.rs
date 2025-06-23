use defuse_crypto::{CryptoHash, Curve, Payload, Secp256k1, SignedPayload, serde::AsCurve};
use impl_tools::autoimpl;
use near_sdk::{env, near};
use serde_with::serde_as;

/// See [ERC-191](https://github.com/ethereum/ercs/blob/master/ERCS/erc-191.md)
#[near(serializers = [json])]
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

impl Payload for Erc191Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        env::keccak256_array(&self.prehash())
    }
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[autoimpl(Deref using self.payload)]
#[derive(Debug, Clone)]
pub struct SignedErc191Payload {
    pub payload: Erc191Payload,

    /// There is no public key member because the public key can be recovered
    /// via `ecrecover()` knowing the data and the signature
    #[serde_as(as = "AsCurve<Secp256k1>")]
    pub signature: <Secp256k1 as Curve>::Signature,
}

impl Payload for SignedErc191Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        self.payload.hash()
    }
}

impl SignedPayload for SignedErc191Payload {
    type PublicKey = <Secp256k1 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        // normalize v field of the signature.
        let signature_v_corrected = if *self.signature.last()? >= 27 {
            let mut sig = self.signature;
            // Ethereum only uses uncompressed keys, with corresponding value v=27/28
            // https://bitcoin.stackexchange.com/a/38909/58790
            *sig.last_mut()? -= 27;
            sig
        } else {
            self.signature
        };
        Secp256k1::verify(&signature_v_corrected, &self.payload.hash(), &())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify() {
        let signed_payload = SignedErc191Payload {
            payload: Erc191Payload("Hello world!".to_string()),
            // Signature constructed in Metamask, using private key: a4b319a82adfc43584e4537fec97a80516e16673db382cd91eba97abbab8ca56
            signature: hex::decode("7800a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e471c").unwrap().try_into().unwrap(),
        };
        signed_payload.verify().unwrap();
    }
}
