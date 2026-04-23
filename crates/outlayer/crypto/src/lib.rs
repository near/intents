pub mod ed25519;
pub mod secp256k1;

use borsh::BorshSerialize;
use digest::Digest;
use digest_io::IoWrapper;
use sha3::Sha3_256;

pub trait DerivableCurve {
    type Tweak;
    type Signature;

    fn make_tweak(tweak: [u8; 32]) -> Self::Tweak;

    fn derive_tweak(path: impl BorshSerialize) -> Self::Tweak {
        let mut hasher = IoWrapper(Sha3_256::new());
        borsh::to_writer(&mut hasher, &path).expect("borsh");
        Self::make_tweak(hasher.0.finalize().into())
    }
}

// TODO: rename
pub trait DerivablePublicKey: Sized {
    type Curve: DerivableCurve;

    fn derive_from_tweak(&self, tweak: <Self::Curve as DerivableCurve>::Tweak) -> Self;

    fn derive_from_borsh(&self, path: impl BorshSerialize) -> Self {
        self.derive_from_tweak(Self::Curve::derive_tweak(path))
    }
}

pub trait DerivableSigningKey {
    type Curve: DerivableCurve;
    type PublicKey: DerivablePublicKey<Curve = Self::Curve>;

    fn public_key(&self) -> Self::PublicKey;

    fn sign_derive(
        &self,
        tweak: <Self::Curve as DerivableCurve>::Tweak,
        msg: &[u8],
    ) -> <Self::Curve as DerivableCurve>::Signature;
}

// pub trait DerivableKey {
//     type PublicKey;
//     // TODO: type Message?
//     type Signature;
//     type Tweak;

//     fn root_public_key(&self) -> Self::PublicKey;

//     fn tweak(hash: [u8; 32]) -> Self::Tweak;
//     fn derive_public_key(root: Self::PublicKey, tweak: Self::Tweak) -> Self::PublicKey;

//     fn sign_derive(&self, tweak: Self::Tweak, msg: &[u8]) -> Self::Signature;

//     fn verify(public_key: Self::PublicKey, msg: &[u8], sig: Self::Signature) -> bool;
// }

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::*;

    pub fn test_roundtrip<K>(
        root_sk: K,
        verify: impl FnOnce(K::PublicKey, &[u8], <K::Curve as DerivableCurve>::Signature),
    ) where
        K: DerivableSigningKey,
        <K::Curve as DerivableCurve>::Tweak: Copy,
    {
        let tweak = <K::Curve as DerivableCurve>::derive_tweak(());
        let derived_pk = root_sk.public_key().derive_from_tweak(tweak);

        let msg: [u8; 32] = Sha256::digest(b"message").into();

        let signature = root_sk.sign_derive(tweak, &msg);

        verify(derived_pk, &msg, signature);

        // assert!(K::verify(derived_pk, &msg, sig), "invalid signature");
    }
}
