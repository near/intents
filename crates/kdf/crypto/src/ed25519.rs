use ed25519_dalek::{self, Signature, VerifyingKey};

use crate::Curve;

pub struct Ed25519;

impl Curve for Ed25519 {
    type PublicKey = VerifyingKey;

    type Message = [u8];

    type Signature = Signature;

    #[inline]
    fn verify(
        public_key: &Self::PublicKey,
        msg: &Self::Message,
        signature: &Self::Signature,
    ) -> bool {
        cfg_select! {
            near => {
                ::near_sdk::env::ed25519_verify(
                    &signature.to_bytes(),
                    msg,
                    public_key.as_bytes(),
                )
            }
            _ => {
                use ed25519_dalek::Verifier;

                // TODO: strict?
                public_key.verify(msg, signature).is_ok()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        hex!("8565df94b8caab08f28cdd2ee014b800915741d4694fa840e50cca02ae5c6466"),
        hex!("060fab6e0fa2ea8913ef80f6f4c6fd8c0c24c2ac044d73b837d887b1e6f378fa"),
        hex!("e4822e15e5988bf08c80b72f2d1292b7229029f342d42bb9dfe4e230c66c10a6c4a86a47ddc58b1446baedf2f1312294d59638c812082a0124e513d4eb16c40e"),
    )]
    #[case(
        hex!("5231b8bba197e888c447ff6617d33dbb7fa571cdbbfb93f0b845c2293c86a3f0"),
        hex!("15401eb21a14a1f9b21277cd65e4e985e094a465c2939c13b39a56b4043a2cdc"),
        hex!("024068f38742fa99be08b9779745562ba10ce336de5865497fc5442353e355d90c1eb986d04fb70d1031b0a7f7cfe80946e0cd3979316b522fbdb8ed35028f0f"),
    )]
    fn verify_ok(
        #[case] public_key: [u8; PUBLIC_KEY_LENGTH],
        #[case] msg: impl AsRef<[u8]>,
        #[case] signature: [u8; SIGNATURE_LENGTH],
    ) {
        let public_key = VerifyingKey::from_bytes(&public_key).unwrap();
        let signature = Signature::from_bytes(&signature);

        assert!(Ed25519::verify(&public_key, msg.as_ref(), &signature));
    }

    #[rstest]
    #[case(
        hex!("8565df94b8caab08f28cdd2ee014b800915741d4694fa840e50cca02ae5c6466"),
        hex!("94fde20581344b29a34224eadd55ceff65afc94148f550255d36e1de9ec064d0"),
        hex!("e4822e15e5988bf08c80b72f2d1292b7229029f342d42bb9dfe4e230c66c10a6c4a86a47ddc58b1446baedf2f1312294d59638c812082a0124e513d4eb16c40e"),
    )]
    #[case(
        hex!("5231b8bba197e888c447ff6617d33dbb7fa571cdbbfb93f0b845c2293c86a3f0"),
        hex!("060fab6e0fa2ea8913ef80f6f4c6fd8c0c24c2ac044d73b837d887b1e6f378fa"),
        hex!("e4822e15e5988bf08c80b72f2d1292b7229029f342d42bb9dfe4e230c66c10a6c4a86a47ddc58b1446baedf2f1312294d59638c812082a0124e513d4eb16c40e"),
    )]
    #[case(
        hex!("8565df94b8caab08f28cdd2ee014b800915741d4694fa840e50cca02ae5c6466"),
        hex!("060fab6e0fa2ea8913ef80f6f4c6fd8c0c24c2ac044d73b837d887b1e6f378fa"),
        hex!("024068f38742fa99be08b9779745562ba10ce336de5865497fc5442353e355d90c1eb986d04fb70d1031b0a7f7cfe80946e0cd3979316b522fbdb8ed35028f0f"),
    )]
    fn verify_fail(
        #[case] public_key: [u8; PUBLIC_KEY_LENGTH],
        #[case] msg: impl AsRef<[u8]>,
        #[case] signature: [u8; SIGNATURE_LENGTH],
    ) {
        let public_key = VerifyingKey::from_bytes(&public_key).unwrap();
        let signature = Signature::from_bytes(&signature);

        assert!(!Ed25519::verify(&public_key, msg.as_ref(), &signature));
    }
}
