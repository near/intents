use near_sdk::{PanicOnDefault, near};

use crate::{RequestMessage, signature::SigningStandard};

pub trait ContractImpl {
    type SigningStandard: SigningStandard<&'static RequestMessage>;
}

/// Signing standard implemented by the contract.
pub type SS = <Contract as ContractImpl>::SigningStandard;

/// Public key used by the signing standard.
pub type PublicKey = <SS as SigningStandard<&'static RequestMessage>>::PublicKey;

/// State of the contract.
pub type State = crate::State<PublicKey>;

#[rustfmt::skip]
macro_rules! contract_standard {
    (standard = $s:literal, version = $v:literal) => {
        #[near(
            contract_state(key = State::STATE_KEY),
            contract_metadata(
                standard(standard = "wallet", version = "1.0.0"),
                standard(standard = $s,       version = $v),
            ),
        )]
        #[derive(Debug, PanicOnDefault)]
        #[repr(transparent)]
        pub struct Contract(pub(crate) State);
    };
}

#[cfg(feature = "ed25519")]
mod ed25519;
#[cfg(feature = "ed25519")]
contract_standard!(standard = "wallet-ed25519", version = "1.0.0");

#[cfg(feature = "webauthn-ed25519")]
mod webauthn_ed25519;
#[cfg(feature = "webauthn-ed25519")]
contract_standard!(standard = "wallet-webauthn-ed25519", version = "1.0.0");

#[cfg(feature = "webauthn-p256")]
mod webauthn_p256;
#[cfg(feature = "webauthn-p256")]
contract_standard!(standard = "wallet-webauthn-p256", version = "1.0.0");

#[cfg(feature = "no-sign")]
mod no_sign;
#[cfg(feature = "no-sign")]
contract_standard!(standard = "wallet-no-sign", version = "1.0.0");
