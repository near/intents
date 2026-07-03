use p256::ecdsa::{Signature, VerifyingKey};

use crate::{Curve, VerifiableCurve};

pub struct P256;

impl Curve for P256 {
    type PublicKey = VerifyingKey;

    type Signature = Signature;
}

impl VerifiableCurve<[u8; 32]> for P256 {
    #[inline]
    fn verify(
        public_key: &Self::PublicKey,
        prehash: [u8; 32],
        signature: &Self::Signature,
    ) -> bool {
        cfg_select! {
            // TODO: cfg(near)
            _ => {
                use p256::{
                    ecdsa::signature::hazmat::PrehashVerifier,
                    elliptic_curve::scalar::IsHigh,
                };

                // TODO: or not?
                // P-256 is the passkey/WebAuthn curve, and WebAuthn does not require low-S — Apple Secure Enclave and various authenticators routinely emit high-S signatures. This is exactly why Ethereum's P256VERIFY precompile (RIP-7212) deliberately does not enforce low-S. So strict low-S rejection here will break signers that emit high-S.
                if signature.s().is_high().into() {
                    // guard against signature malleability
                    return false;
                }

                public_key.verify_prehash(&prehash, signature).is_ok()
            }
        }
    }
}
