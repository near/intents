use defuse_crypto::{CryptoHash, Curve, Payload, Secp256k1, SignedPayload, serde::AsCurve};
use impl_tools::autoimpl;
use near_sdk::{env, near};
use serde_with::serde_as;

/// See [TIP-191](https://github.com/tronprotocol/tips/blob/master/tip-191.md)
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct Tip191Payload(pub String);

impl Tip191Payload {
    #[inline]
    pub fn prehash(&self) -> Vec<u8> {
        let data = self.0.as_bytes();
        [
            // Prefix not specified in the standard. But from: https://tronweb.network/docu/docs/Sign%20and%20Verify%20Message/
            format!("\x19TRON Signed Message:\n{}", data.len()).as_bytes(),
            data,
        ]
        .concat()
    }
}

impl Payload for Tip191Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        env::keccak256_array(&self.prehash())
    }
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[autoimpl(Deref using self.payload)]
#[derive(Debug, Clone)]
pub struct SignedTip191Payload {
    pub payload: Tip191Payload,

    /// There is no public key member because the public key can be recovered
    /// via `ecrecover()` knowing the data and the signature
    #[serde_as(as = "AsCurve<Secp256k1>")]
    pub signature: <Secp256k1 as Curve>::Signature,
}

impl Payload for SignedTip191Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        self.payload.hash()
    }
}

impl SignedPayload for SignedTip191Payload {
    type PublicKey = <Secp256k1 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Secp256k1::verify(&self.signature, &self.payload.hash(), &())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use defuse_test_utils::{
        random::{Rng, rng},
        tamper::{tamper_bytes, tamper_string},
    };
    use rstest::rstest;

    fn fix_v_in_signature(mut sig: [u8; 65]) -> [u8; 65] {
        if *sig.last().unwrap() >= 27 {
            // Ethereum only uses uncompressed keys, with corresponding value v=27/28
            // https://bitcoin.stackexchange.com/a/38909/58790
            *sig.last_mut().unwrap() -= 27;
        }
        sig
    }

    #[test]
    fn verify() {
        let msg = "Hello, TRON!";

        // Signature constructed in TronLink, using private key: a4b319a82adfc43584e4537fec97a80516e16673db382cd91eba97abbab8ca56
        let signature = hex_literal::hex!(
            "eea1651a60600ec4d9c45e8ae81da1a78377f789f0ac2019de66ad943459913015ef9256809ee0e6bb76e303a0b4802e475c1d26ade5d585292b80c9fe9cb10c1c"
        );
        let signature = fix_v_in_signature(signature);

        // Public key can be derived using `ethers_signers` crate:
        // let wallet = LocalWallet::from_str(
        //     "a4b319a82adfc43584e4537fec97a80516e16673db382cd91eba97abbab8ca56",
        // )?;
        // let signing_key = wallet.signer();
        // let verifying_key = signing_key.verifying_key();
        // let public_key = verifying_key.to_encoded_point(false);
        // // Notice that we skip the first byte, 0x04
        // println!("Public key: 0x{}", hex::encode(public_key.as_bytes()[1..]));

        let public_key = hex_literal::hex!(
            "85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"
        );

        let signed_payload = SignedTip191Payload {
            payload: Tip191Payload(msg.to_string()),
            signature,
        };

        assert_eq!(signed_payload.verify(), Some(public_key));
    }

    #[rstest]
    fn tamper_message_fails(mut rng: impl Rng) {
        let msg = "Hello, TRON!";

        let signature = hex_literal::hex!(
            "eea1651a60600ec4d9c45e8ae81da1a78377f789f0ac2019de66ad943459913015ef9256809ee0e6bb76e303a0b4802e475c1d26ade5d585292b80c9fe9cb10c1c"
        );
        let signature = fix_v_in_signature(signature);

        let public_key = hex_literal::hex!(
            "85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"
        );

        {
            let signed_payload = SignedTip191Payload {
                payload: Tip191Payload(msg.to_string()),
                signature,
            };

            assert_eq!(signed_payload.verify(), Some(public_key));
        }

        {
            let bad_signed_payload = SignedTip191Payload {
                payload: Tip191Payload(tamper_string(&mut rng, msg)),
                signature,
            };

            assert_ne!(bad_signed_payload.verify(), Some(public_key));
        }
    }

    #[rstest]
    fn tamper_signature_fails(mut rng: impl Rng) {
        let msg = "Hello, TRON!";

        let signature = hex_literal::hex!(
            "eea1651a60600ec4d9c45e8ae81da1a78377f789f0ac2019de66ad943459913015ef9256809ee0e6bb76e303a0b4802e475c1d26ade5d585292b80c9fe9cb10c1c"
        );
        let signature = fix_v_in_signature(signature);

        let public_key = hex_literal::hex!(
            "85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"
        );

        {
            let signed_payload = SignedTip191Payload {
                payload: Tip191Payload(msg.to_string()),
                signature,
            };

            assert_eq!(signed_payload.verify(), Some(public_key));
        }

        {
            let bad_signed_payload = SignedTip191Payload {
                payload: Tip191Payload(msg.to_string()),
                signature: tamper_bytes(&mut rng, &signature, false)
                    .try_into()
                    .unwrap(),
            };

            assert_ne!(bad_signed_payload.verify(), Some(public_key));
        }
    }
}
