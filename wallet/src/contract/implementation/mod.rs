use core::ops::{Deref, DerefMut};

use near_sdk::{PanicOnDefault, near};

use crate::{RequestMessage, signature::SigningStandard};

pub trait ContractImpl {
    type SigningStandard: SigningStandard<&'static RequestMessage>;
}

/// Signing standard implemented by the contract.
type SS = <Contract as ContractImpl>::SigningStandard;

/// Public key used by the signing standard.
type PublicKey = <SS as SigningStandard<&'static RequestMessage>>::PublicKey;

/// State of the contract.
type State = crate::State<PublicKey>;

/// `#[near(contract_metadata(standard(...)))]` macro doesn't support
/// adding more standards in separate attributes. So, we have to combine
/// them all depending on a specific feature enabled.
macro_rules! contract {
    ($(
        #[cfg_attr(
            feature = $feature:literal,
            near(contract_metadata(standard(standard = $s:literal, version = $v:literal)))
        )]
        mod $mod:ident;
    )+) => {
        $(
            #[cfg(feature = $feature)]
            mod $mod;
        )+

        /// By default, the contract implements `no-sign`, i.e. always
        /// rejects signature.
        #[cfg(not(any($(feature = $feature),+)))]
        const _: () = {
            use crate::signature::no_sign::NoSign;

            impl ContractImpl for Contract {
                type SigningStandard = NoSign;
            }
        };

        $(#[cfg_attr(
            feature = $feature,
            near(
                contract_state(key = State::STATE_KEY),
                contract_metadata(
                    standard(standard = "wallet", version = "1.0.0"),
                    standard(standard = $s,       version = $v),
                ),
            )
        )])+
        #[cfg_attr(
            not(any($(feature = $feature),+)),
            near(
                contract_state(key = State::STATE_KEY),
                contract_metadata(
                    standard(standard = "wallet", version = "1.0.0"),
                    standard(standard = "wallet-no-sign", version = "1.0.0"),
                ),
            )
        )]
        #[derive(Debug, PanicOnDefault)]
        #[repr(transparent)]
        pub struct Contract(pub(crate) State);
    };
}

contract! {
    #[cfg_attr(
        feature = "ed25519",
        near(contract_metadata(
            standard(standard = "wallet-ed25519", version = "1.0.0")
        ))
    )]
    mod ed25519;

    #[cfg_attr(
        feature = "webauthn-ed25519",
        near(contract_metadata(
            standard(standard = "wallet-webauthn-ed25519", version = "1.0.0")
        ))
    )]
    mod webauthn_ed25519;

    #[cfg_attr(
        feature = "webauthn-p256",
        near(contract_metadata(
            standard(standard = "wallet-webauthn-p256", version = "1.0.0")
        ))
    )]
    mod webauthn_p256;
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
