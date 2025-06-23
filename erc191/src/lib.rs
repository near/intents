use defuse_crypto::{CryptoHash, Curve, Payload, Secp256k1, SignedPayload, serde::AsCurve};
use impl_tools::autoimpl;
use near_sdk::{env, near};
use serde_with::serde_as;

/// See [ERC-191](https://github.com/ethereum/ercs/blob/master/ERCS/erc-191.md)
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct Erc191Payload(pub String);

impl Erc191Payload {
    #[inline]
    pub fn prehash(&self) -> Vec<u8> {
        let data = self.0.as_bytes();
        [
            format!("\x19Ethereum Signed Message:\n{}", data.len()).as_bytes(),
            data,
        ]
        .concat()
    }
}

impl Payload for Erc191Payload {
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
pub struct SignedErc191Payload {
    pub payload: Erc191Payload,

    /// There is no public key member because the public key can be recovered
    /// via `ecrecover()` knowing the data and the signature
    #[serde_as(as = "AsCurve<Secp256k1>")]
    pub signature: <Secp256k1 as Curve>::Signature,
}

impl Payload for SignedErc191Payload {
    #[inline]
    fn hash(&self) -> CryptoHash {
        self.payload.hash()
    }
}

impl SignedPayload for SignedErc191Payload {
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
        let msg = "Hello world!";

        // Signature constructed in Metamask, using private key: a4b319a82adfc43584e4537fec97a80516e16673db382cd91eba97abbab8ca56
        let signature = hex_literal::hex!(
            "7800a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e471c"
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

        let signed_payload = SignedErc191Payload {
            payload: Erc191Payload(msg.to_string()),
            signature,
        };

        assert_eq!(signed_payload.verify().unwrap(), public_key);
    }

    #[rstest]
    fn tamper_message_fails(mut rng: impl Rng) {
        let msg = "Hello world!";

        // Signature constructed in Metamask, using private key: a4b319a82adfc43584e4537fec97a80516e16673db382cd91eba97abbab8ca56
        let signature = hex_literal::hex!(
            "7800a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e471c"
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

        {
            let signed_payload = SignedErc191Payload {
                payload: Erc191Payload(msg.to_string()),
                signature,
            };

            assert_eq!(signed_payload.verify().unwrap(), public_key);
        }

        {
            let bad_signed_payload = SignedErc191Payload {
                payload: Erc191Payload(tamper_string(&mut rng, msg)),
                signature,
            };

            assert_ne!(bad_signed_payload.verify(), Some(public_key));
        }
    }

    #[rstest]
    fn tamper_signature_fails(mut rng: impl Rng) {
        let msg = "Hello world!";

        // Signature constructed in Metamask, using private key: a4b319a82adfc43584e4537fec97a80516e16673db382cd91eba97abbab8ca56
        let signature = hex_literal::hex!(
            "7800a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e471c"
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

        {
            let signed_payload = SignedErc191Payload {
                payload: Erc191Payload(msg.to_string()),
                signature,
            };

            assert_eq!(signed_payload.verify().unwrap(), public_key);
        }

        {
            let bad_signed_payload = SignedErc191Payload {
                payload: Erc191Payload(msg.to_string()),
                signature: tamper_bytes(&mut rng, &signature, false)
                    .try_into()
                    .unwrap(),
            };

            assert_ne!(bad_signed_payload.verify(), Some(public_key));
        }
    }
}
