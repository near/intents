use core::ops::{Deref, DerefMut};
use std::fmt::Display;

use near_sdk::{
    PanicOnDefault,
    borsh::{BorshDeserialize, BorshSerialize},
    near,
};

use crate::{
    STATE_KEY,
    signature::{RequestMessage, SigningStandard},
};

pub trait ContractImpl {
    /// Signing standard implementation of the contract
    type SigningStandard: SigningStandard<
            &'static RequestMessage,
            PublicKey: BorshSerialize + BorshDeserialize + Display,
        >;
}

/// Signing standard implemented by the contract
type SS = <Contract as ContractImpl>::SigningStandard;

/// Public key used by the signing standard
type PublicKey = <SS as SigningStandard<&'static RequestMessage>>::PublicKey;

/// State of the contract
type State = crate::State<PublicKey>;

/// `#[near(contract_metadata(standard(...)))]` macro doesn't support
/// adding more standards in separate attributes. So, we have to combine
/// them all depending on a specific feature enabled.
macro_rules! contract_impl {
    (@meta $(
        #[cfg_attr(
            $meta:meta,
            near(contract_metadata(
                standard(standard = $s:literal, version = $v:literal)
            ))
        )] $block:block
    )+) => {
        $(
            #[cfg($meta)]
            const _: () = $block;
        )+

        $(#[cfg_attr(
            $meta,
            near(
                contract_state(key = STATE_KEY),
                contract_metadata(
                    standard(standard = "wallet",       version = "1.0.0"),
                    standard(standard = $s,             version = $v     ),
                ),
            )
        )])+
        #[derive(Debug, PanicOnDefault)]
        #[repr(transparent)]
        pub struct Contract(pub(crate) State);
    };
    ($(
        #[cfg_attr(
            feature = $feature:literal,
            near(contract_metadata(
                standard(standard = $s:literal, version = $v:literal)
            ))
        )] $block:block
    )+) => {
        contract_impl!{
            @meta $(
                #[cfg_attr(
                    feature = $feature,
                    near(contract_metadata(
                        standard(standard = $s, version = $v)
                    ))
                )] $block
            )+
            // When no signing features enabled, the contract always rejects the
            // signature. This can be useful to deploy "1-of-M multisig"/
            // "fan-out" wallet, where extensions are pre-defined at the
            // initialization stage (i.e. state_init). So only extensions
            // can execute requests via `w_execute_extension()`.
            #[cfg_attr(
                not(any($(feature = $feature),+)),
                near(contract_metadata(
                    standard(standard = "wallet-no-sign", version = "1.0.0")
                ))
            )] {
                use crate::signature::no_sign::NoSign;

                impl ContractImpl for Contract {
                    /// Always reject the signature.
                    type SigningStandard = NoSign;
                }
            }
        }
    };
}

contract_impl! {
    #[cfg_attr(
        feature = "ed25519",
        near(contract_metadata(
            standard(standard = "wallet-ed25519", version = "1.0.0")
        ))
    )] {
        use defuse_crypto::Ed25519;

        use crate::signature::{Borsh, DomainPrefix, Sha256};

        impl ContractImpl for Contract {
            type SigningStandard = Borsh<DomainPrefix<Sha256<Ed25519>>>;
        }
    }

    #[cfg_attr(
        feature = "webauthn-ed25519",
        near(contract_metadata(
            standard(standard = "wallet-webauthn-ed25519", version = "1.0.0")
        ))
    )] {
        use crate::signature::{
            Borsh, DomainPrefix, Sha256,
            webauthn::{Ed25519, Webauthn},
        };

        impl ContractImpl for Contract {
            /// Webauthn [COSE EdDSA (-8) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
            /// ed25519 curve.
            ///
            /// We hash the payload for webauthn, since:
            /// 1. Authenticators are general-purpose signers and they usually implement
            ///   blind singing.
            /// 2. This reduces length of the `proof` submitted on-chain.
            type SigningStandard = Borsh<DomainPrefix<Sha256<Webauthn<Ed25519>>>>;
        }
    }

    #[cfg_attr(
        feature = "webauthn-p256",
        near(contract_metadata(
            standard(standard = "wallet-webauthn-p256", version = "1.0.0")
        ))
    )] {
        use crate::signature::{
            Borsh, DomainPrefix, Sha256,
            webauthn::{P256, Webauthn},
        };

        impl ContractImpl for Contract {
            /// [COSE ES256 (-7) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
            /// P256 (a.k.a secp256r1) over SHA-256
            ///
            /// We hash the payload for webauthn, since:
            /// 1. Authenticators are general-purpose signers and they usually implement
            ///   blind singing.
            /// 2. This reduces length of the `proof` submitted on-chain.
            type SigningStandard = Borsh<DomainPrefix<Sha256<Webauthn<P256>>>>;
        }
    }

    #[cfg_attr(
        feature = "erc191",
        near(contract_metadata(
            standard(standard = "wallet-erc191", version = "1.0.0")
        ))
    )] {
        use crate::signature::erc191::Erc191;

        impl ContractImpl for Contract {
            type SigningStandard = Erc191;
        }
    }
}

impl Deref for Contract {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Contract {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
