use core::fmt::Display;

use borsh::{BorshDeserialize, BorshSerialize};

use crate::RequestMessage;

pub trait WalletSignatureSchema {
    // TODO: trait bounds?
    type PublicKey;

    fn verify(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool;
}

type Proof = String;

pub trait WalletSigner<S: WalletSignatureSchema> {
    type Error;

    fn public_key(&self) -> S::PublicKey;
    fn sign(&self, msg: &RequestMessage) -> Result<Proof, Self::Error>;
}
