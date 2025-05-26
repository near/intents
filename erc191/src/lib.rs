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
        Secp256k1::verify(&self.signature, &self.payload.hash(), &())
    }
}
