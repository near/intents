// mod ed25519; // TODO

use crate::RequestMessage;

pub trait WalletSignatureSchema {
    type PublicKey;

    fn verify(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str);
}

type Proof = String;

pub trait WalletSigner<S: WalletSignatureSchema> {
    type Error;

    fn public_key(&self) -> S::PublicKey;
    fn sign(&self, msg: &RequestMessage) -> Result<Proof, Self::Error>;
}
