use crate::signature::no_sign::NoSign;

use super::{Contract, ContractImpl};

impl ContractImpl for Contract {
    type SigningStandard = NoSign;
}
