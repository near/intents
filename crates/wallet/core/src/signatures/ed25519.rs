use crate::signatures::WalletSignatureSchema;

pub struct Ed25519;

impl WalletSignatureSchema for Ed25519 {
    type PublicKey;

    fn verify(public_key: &Self::PublicKey, msg: &crate::RequestMessage, proof: &str) {
        
    }
}