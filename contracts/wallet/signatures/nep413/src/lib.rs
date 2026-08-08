#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "signer")]
mod signer;
#[cfg(feature = "signer")]
pub use self::signer::*;

use core::{marker::PhantomData, str::FromStr};

pub use defuse_crypto as crypto;
use defuse_crypto::Curve;
pub use defuse_nep413 as nep413;
use defuse_nep413::{Nep413, Nep413Payload};
use defuse_wallet::{RequestMessage, SignatureSchema, offchain::OffchainMessage};

/// Wallet [signature schema](SignatureSchema) using the [NEP-413](Nep413)
/// signing envelope and curve `C`.
///
/// The payload is reconstructed with no callback URL, so signatures made with
/// a callback URL are intentionally rejected.
pub struct WalletNep413<C: WalletNep413Curve>(PhantomData<C>);

impl<C> WalletNep413<C>
where
    C: WalletNep413Curve,
    <C as Curve>::Signature: TryFrom<C::Proof>,
    for<'a> <C as Curve>::PublicKey: TryFrom<&'a C::StoredPublicKey>,
{
    fn verify_payload(
        public_key: &C::StoredPublicKey,
        payload: &Nep413Payload,
        proof: &str,
    ) -> bool {
        let Ok(proof) = C::Proof::from_str(proof) else {
            return false;
        };

        let Ok(signature) = <C as Curve>::Signature::try_from(proof) else {
            return false;
        };

        let Ok(public_key) = <C as Curve>::PublicKey::try_from(public_key) else {
            return false;
        };

        Nep413::verify::<C>(&public_key, payload, &signature)
    }
}

impl<C> SignatureSchema for WalletNep413<C>
where
    C: WalletNep413Curve,
    <C as Curve>::Signature: TryFrom<C::Proof>,
    for<'a> <C as Curve>::PublicKey: TryFrom<&'a C::StoredPublicKey>,
{
    type PublicKey = C::StoredPublicKey;

    #[inline]
    fn verify_request_msg(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool {
        Self::verify_payload(public_key, &msg.clone().into_nep413_payload(None), proof)
    }

    #[inline]
    fn verify_offchain_msg(
        public_key: &Self::PublicKey,
        msg: &OffchainMessage,
        proof: &str,
    ) -> bool {
        Self::verify_payload(public_key, &msg.clone().into_nep413_payload(None), proof)
    }
}

/// Curve adapter used by [`WalletNep413`] to define its contract-facing public
/// key and proof representations.
pub trait WalletNep413Curve: Curve {
    /// Public key stored in wallet contract state.
    type StoredPublicKey;

    /// Signature representation parsed from the string proof passed to the
    /// wallet contract.
    type Proof: FromStr;
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use defuse_crypto::ed25519::{
        Ed25519PublicKey, Ed25519Signature,
        ed25519_dalek::{Signer as _, SigningKey},
    };
    use defuse_wallet::{Request, Timestamp, WalletOp};

    use crate::ed25519::Ed25519;

    use super::*;

    type Schema = WalletNep413<Ed25519>;

    fn signer() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    fn request_msg() -> RequestMessage {
        RequestMessage {
            pay_for_gas: false,
            chain_id: "mainnet".to_string(),
            signer_id: "wallet.near".parse().unwrap(),
            nonce: 42,
            created_at: "2026-08-08T12:34:56.123456789Z".parse().unwrap(),
            timeout: Duration::from_hours(1),
            request: Request::from(WalletOp::add_extension(
                "extension.near"
                    .parse::<defuse_wallet::AccountId>()
                    .unwrap(),
            )),
        }
    }

    fn sign_payload(signer: &SigningKey, payload: &Nep413Payload) -> String {
        Ed25519Signature::from(signer.sign(&Nep413::prehash(payload))).to_string()
    }

    fn public_key(signer: &SigningKey) -> Ed25519PublicKey {
        signer.verifying_key().into()
    }

    #[test]
    fn verifies_typed_nep413_proof() {
        let signer = signer();
        let msg = request_msg();
        let proof = sign_payload(&signer, &msg.clone().into_nep413_payload(None));

        assert!(proof.starts_with("ed25519:"));
        assert!(Schema::verify_request_msg(
            &public_key(&signer),
            &msg,
            &proof,
        ));
    }

    #[test]
    fn rejects_tampered_request_message() {
        let signer = signer();
        let msg = request_msg();
        let proof = sign_payload(&signer, &msg.clone().into_nep413_payload(None));

        let mut tampered = msg.clone();
        tampered.pay_for_gas = true;
        assert!(!Schema::verify_request_msg(
            &public_key(&signer),
            &tampered,
            &proof,
        ));

        let mut tampered = msg.clone();
        tampered.chain_id = "testnet".to_string();
        assert!(!Schema::verify_request_msg(
            &public_key(&signer),
            &tampered,
            &proof,
        ));

        let mut tampered = msg.clone();
        tampered.nonce += 1;
        assert!(!Schema::verify_request_msg(
            &public_key(&signer),
            &tampered,
            &proof,
        ));

        let mut tampered = msg;
        tampered.request = Request::from(WalletOp::disable_signature());
        assert!(!Schema::verify_request_msg(
            &public_key(&signer),
            &tampered,
            &proof,
        ));
    }

    #[test]
    fn rejects_callback_url_payload() {
        let signer = signer();
        let msg = request_msg();
        let proof = sign_payload(
            &signer,
            &msg.clone()
                .into_nep413_payload("https://wallet.example/callback".to_string()),
        );

        assert!(!Schema::verify_request_msg(
            &public_key(&signer),
            &msg,
            &proof,
        ));
    }

    #[test]
    fn raw_and_nep413_schemas_are_not_interchangeable() {
        use defuse_wallet_ed25519::WalletEd25519;

        let signer = signer();
        let msg = request_msg();
        let nep413_proof = sign_payload(&signer, &msg.clone().into_nep413_payload(None));
        let raw_proof = Ed25519Signature::from(signer.sign(&msg.hash())).to_string();
        let public_key = public_key(&signer);

        assert!(!WalletEd25519::verify_request_msg(
            &public_key,
            &msg,
            &nep413_proof,
        ));
        assert!(!Schema::verify_request_msg(&public_key, &msg, &raw_proof));
    }

    #[test]
    fn verifies_offchain_message_and_rejects_tampering() {
        let signer = signer();
        let mut msg = OffchainMessage {
            chain_id: "mainnet".to_string(),
            signer_id: "wallet.near".parse().unwrap(),
            path: vec!["resolver.near".parse().unwrap()],
            timestamp: Timestamp::UNIX_EPOCH,
            payload: "authorize me".to_string(),
        };
        let proof = sign_payload(&signer, &msg.clone().into_nep413_payload(None));
        let public_key = public_key(&signer);

        assert!(Schema::verify_offchain_msg(&public_key, &msg, &proof));

        msg.payload.push('!');
        assert!(!Schema::verify_offchain_msg(&public_key, &msg, &proof));
    }

    #[test]
    fn rejects_malformed_proof() {
        assert!(!Schema::verify_request_msg(
            &public_key(&signer()),
            &request_msg(),
            "not-a-signature",
        ));
    }
}
