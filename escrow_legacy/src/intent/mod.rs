use std::collections::BTreeSet;

use near_sdk::AccountId;

pub struct Escrow {
    pub inputs: BTreeSet<Asset>,
    // todo: add recepinets?
    pub output: BTreeSet<Asset>,
}

pub struct Asset {
    
}


pub struct Escrow {
    pub maker: Option<AccountId>,
    pub taker: Option<AccountId>,
    // todo: add recepinets?
    pub output: BTreeSet<Asset>,
}