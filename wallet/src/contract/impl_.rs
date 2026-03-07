use core::ops::{Deref, DerefMut};

use near_sdk::{PanicOnDefault, near};

use crate::{
    Nonces, STATE_KEY,
    signature::{RequestMessage, SigningStandard},
};

// #[cfg(not(feature = "highload"))]
// type Nonces = crate::Seqno;
// #[cfg(feature = "highload")]
// type Nonces = crate::HighloadNonces;

// pub type RequestMessage = crate::RequestMessage<Nonces>;

pub trait NoncesImpl {
    type Nonces: Nonces + 'static;
}

#[cfg(not(feature = "highload"))]
const _: () = {
    use crate::Seqno;

    impl NoncesImpl for Contract {
        type Nonces = Seqno;
    }
};

#[cfg(feature = "highload")]
const _: () = {
    use crate::HighloadNonces;

    impl NoncesImpl for Contract {
        type Nonces = HighloadNonces;
    }
};

/// Nonces implemented by the contract.
type ContractNonces<C> = <C as NoncesImpl>::Nonces;
pub type ContractNonce<C> = <ContractNonces<C> as Nonces>::Nonce;
pub type ContractNonceError<C> = <ContractNonces<C> as Nonces>::Error;

pub type ContractRequestMessage<C> = RequestMessage<ContractNonce<C>>;

pub trait SigningStandardImpl: NoncesImpl {
    type SigningStandard: SigningStandard<&'static ContractRequestMessage<Self>>;
}
/// Signing standard implemented by the contract
type ContractSigningStandard<C> = <C as SigningStandardImpl>::SigningStandard;

/// Public key used by the signing standard.
type ContractPubKey<C> =
    <ContractSigningStandard<C> as SigningStandard<&'static ContractRequestMessage<C>>>::PublicKey;

/// State of the contract.
type ContractState<C> = crate::State<ContractPubKey<C>, ContractNonces<C>>;

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
            all(not(feature = "highload"), $meta),
            near(
                contract_state(key = STATE_KEY),
                contract_metadata(
                    standard(standard = "wallet",       version = "1.0.0"),
                    standard(standard = "wallet-seqno", version = "1.0.0"),
                    standard(standard = $s,             version = $v     ),
                ),
            )
        )])+
        $(#[cfg_attr(
            all(feature = "highload", $meta),
            near(
                contract_state(key = STATE_KEY),
                contract_metadata(
                    standard(standard = "wallet",          version = "1.0.0"),
                    standard(standard = "wallet-highload", version = "1.0.0"),
                    standard(standard = $s,                version = $v     ),
                ),
            )
        )])+
        #[derive(Debug, PanicOnDefault)]
        #[repr(transparent)]
        pub struct Contract(pub(crate) ContractState<Self>);
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
            #[cfg_attr(
                not(any($(feature = $feature),+)),
                near(contract_metadata(
                    standard(standard = "wallet-no-sign", version = "1.0.0")
                ))
            )] {
                use crate::signature::no_sign::NoSign;

                impl SigningStandardImpl for Contract {
                    /// By default, the contract implements `no-sign`, i.e. always
                    /// rejects signature.
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

        impl SigningStandardImpl for Contract {
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

        impl SigningStandardImpl for Contract {
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
            Borsh, Sha256, DomainPrefix,
            webauthn::{P256, Webauthn},
        };

        impl SigningStandardImpl for Contract {
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
}

impl Deref for Contract {
    type Target = ContractState<Self>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Contract {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
