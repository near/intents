use defuse_crypto::{CurveTypes, Secp256k1};
use impl_tools::autoimpl;

/// See [TIP-191](https://github.com/tronprotocol/tips/blob/master/tip-191.md)
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
pub struct SignedTip191Payload {
    pub payload: Tip191Payload,

    /// There is no public key member because the public key can be recovered
    /// via `ecrecover()` knowing the data and the signature
    #[cfg_attr(
        feature = "serde",
        serde_as(as = "defuse_crypto::serde::AsCurve<Secp256k1>")
    )]
    pub signature: <Secp256k1 as CurveTypes>::Signature,
}
