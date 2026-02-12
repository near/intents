use defuse_crypto::Ed25519;

use crate::signature::Borsh;

use super::{Contract, ContractImpl};

impl ContractImpl for Contract {
    // TODO: domain prefix
    type SigningStandard = Borsh<Ed25519>;
}
