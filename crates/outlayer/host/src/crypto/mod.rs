pub mod ed25519;
pub mod secp256k1;

use self::{ed25519::Ed25519Host, secp256k1::Secp256k1Host};

pub trait CryptoHost: Ed25519Host + Secp256k1Host {}
impl<T> CryptoHost for T where T: Ed25519Host + Secp256k1Host {}
