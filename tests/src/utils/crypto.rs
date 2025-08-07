use defuse::core::{
    crypto::Payload,
    nep413::{Nep413Payload, SignedNep413Payload},
    sep53::{Sep53Payload, SignedSep53Payload},
    ton_connect::{SignedTonConnectPayload, TonConnectPayload},
};
use defuse_bip322::{Address, SignedBip322Payload};
use near_workspaces::Account;

pub trait Signer {
    fn secret_key(&self) -> near_crypto::SecretKey;

    fn sign_nep413(&self, payload: Nep413Payload) -> SignedNep413Payload;
    fn sign_ton_connect(&self, payload: TonConnectPayload) -> SignedTonConnectPayload;
    fn sign_sep53(&self, payload: Sep53Payload) -> SignedSep53Payload;
    fn sign_bip322(&self, message: String) -> SignedBip322Payload;
}

impl Signer for Account {
    fn secret_key(&self) -> near_crypto::SecretKey {
        // near_sdk does not expose near_crypto API
        self.secret_key().to_string().parse().unwrap()
    }

    fn sign_nep413(&self, payload: Nep413Payload) -> SignedNep413Payload {
        let secret_key = Signer::secret_key(self);

        match (secret_key.sign(&payload.hash()), secret_key.public_key()) {
            (near_crypto::Signature::ED25519(sig), near_crypto::PublicKey::ED25519(pk)) => {
                SignedNep413Payload {
                    payload,
                    public_key: pk.0,
                    signature: sig.to_bytes(),
                }
            }
            _ => unreachable!(),
        }
    }

    fn sign_ton_connect(&self, payload: TonConnectPayload) -> SignedTonConnectPayload {
        let secret_key = Signer::secret_key(self);

        match (secret_key.sign(&payload.hash()), secret_key.public_key()) {
            (near_crypto::Signature::ED25519(sig), near_crypto::PublicKey::ED25519(pk)) => {
                SignedTonConnectPayload {
                    payload,
                    public_key: pk.0,
                    signature: sig.to_bytes(),
                }
            }
            _ => unreachable!(),
        }
    }

    fn sign_sep53(&self, payload: Sep53Payload) -> SignedSep53Payload {
        let secret_key = Signer::secret_key(self);

        match (secret_key.sign(&payload.hash()), secret_key.public_key()) {
            (near_crypto::Signature::ED25519(sig), near_crypto::PublicKey::ED25519(pk)) => {
                SignedSep53Payload {
                    payload,
                    public_key: pk.0,
                    signature: sig.to_bytes(),
                }
            }
            _ => unreachable!(),
        }
    }

    fn sign_bip322(&self, message: String) -> SignedBip322Payload {
        // For testing purposes, create a dummy BIP-322 signature
        // In a real implementation, this would need proper Bitcoin ECDSA signing

        // Create a dummy P2WPKH address for testing
        // Using a valid mainnet address format: bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4
        let address: Address = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"
            .parse()
            .unwrap_or_else(|_| {
                // Fallback: create P2PKH with dummy data if parsing fails
                Address::P2PKH {
                    pubkey_hash: [0u8; 20],
                }
            });

        // Create empty witness (signature verification will fail, but structure is correct for testing)
        let signature = address.create_empty_witness();

        SignedBip322Payload {
            address,
            message,
            signature,
        }
    }
}
