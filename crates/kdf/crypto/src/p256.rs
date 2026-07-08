use p256::ecdsa::{Signature, SigningKey, VerifyingKey, signature::hazmat::PrehashSigner};
pub use p256::*;

use crate::{Curve, Signer};

pub struct P256;

impl Curve for P256 {
    type PublicKey = VerifyingKey;

    type Signature = Signature;

    // TODO: docs: prehash
    #[inline]
    fn verify(public_key: &Self::PublicKey, prehash: &[u8], signature: &Self::Signature) -> bool {
        // accept only 32 byte prehash
        let Ok(prehash) = <&[u8; 32]>::try_from(prehash) else {
            return false;
        };

        cfg_select! {
            // TODO: cfg(near)
            _ => {{
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

                public_key.verify_prehash(prehash, signature).is_ok()
            }}
        }
    }
}

impl Signer<P256> for SigningKey {
    type Error = Error;

    fn public_key(&self) -> <P256 as Curve>::PublicKey {
        *self.verifying_key()
    }

    fn sign(&self, msg: &[u8]) -> Result<<P256 as Curve>::Signature, Self::Error> {
        self.sign_prehash(msg)
            .map_err(|_| Error::InvalidPrehashLength)
    }
}

pub enum Error {
    InvalidPrehashLength,
}

// TODO: tests
