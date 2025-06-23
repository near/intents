use defuse_crypto::{CryptoHash, Curve, Payload, Secp256k1, SignedPayload, serde::AsCurve};
use impl_tools::autoimpl;
use near_sdk::{env, near};
use serde_with::serde_as;

/// See [TIP-191](https://github.com/tronprotocol/tips/blob/master/tip-191.md)
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct Tip191Payload(pub String);

impl Tip191Payload {
    #[inline]
    pub fn prehash(&self) -> Vec<u8> {
        let data = self.0.as_bytes();
        [
            // Prefix not specified in the standard. But from: https://tronweb.network/docu/docs/Sign%20and%20Verify%20Message/
            format!("\x19TRON Signed Message:\n{}", data.len()).as_bytes(),
            data,
        ]
        .concat()
    }
}

impl Payload for Tip191Payload {
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
pub struct SignedTip191Payload {
    pub payload: Tip191Payload,

    /// There is no public key member because the public key can be recovered
    /// via `ecrecover()` knowing the data and the signature
    #[serde_as(as = "AsCurve<Secp256k1>")]
    pub signature: <Secp256k1 as Curve>::Signature,
}

impl Payload for SignedTip191Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
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
