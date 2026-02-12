use crate::signature::{
    Borsh, Sha256,
    webauthn::{P256, Webauthn},
};

use super::{Contract, ContractImpl};

impl ContractImpl for Contract {
    /// Webauthn [COSE EdDSA (-8) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
    /// ed25519 curve.
    ///
    /// We use `hash(borsh(...))` for webauthn, since:
    /// 1. Authenticators are general-purpose signers and they usually implement
    ///   blind singing.
    /// 2. This reduces length of the `proof` submitted on-chain.
    type SigningStandard = Borsh<Sha256<Webauthn<P256>>>;
}
