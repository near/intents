pub mod ed25519;
pub mod secp256k1;

pub trait DerivableKey {
    type PublicKey;
    type Signature;
    type Tweak;

    fn root_public_key(&self) -> Self::PublicKey;

    fn tweak(hash: [u8; 32]) -> Self::Tweak;
    fn derive_public_key(root: Self::PublicKey, tweak: Self::Tweak) -> Self::PublicKey;

    fn sign(&self, tweak: Self::Tweak, msg: &[u8]) -> Self::Signature;

    fn verify(public_key: Self::PublicKey, msg: &[u8], sig: Self::Signature) -> bool;
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::*;

    pub fn test_roundtrip<K>(root_sk: K)
    where
        K: DerivableKey,
        K::Tweak: Copy,
    {
        let tweak = K::tweak([42u8; 32]);
        let root_pk = root_sk.root_public_key();
        let derived_pk = K::derive_public_key(root_pk, tweak);

        let msg: [u8; 32] = Sha256::digest(b"message").into();

        let sig = root_sk.sign(tweak, &msg);

        assert!(K::verify(derived_pk, &msg, sig), "invalid signature");
    }
}
