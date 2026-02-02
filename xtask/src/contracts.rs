use clap::{Args, ValueEnum};

#[derive(Args, Clone, Default, Debug)]
pub struct ContractOptions {
    #[arg(short, long)]
    pub contract: Contract,
    pub features: Option<String>,
}

impl ContractOptions {
    pub const fn new_without_features(contract: Contract) -> Self {
        Self {
            contract,
            features: None,
        }
    }

    pub fn all_without_features() -> Vec<Self> {
        Contract::all()
            .into_iter()
            .map(|c| Self {
                contract: c,
                features: None,
            })
            .collect()
    }
}

pub struct ContractSpec {
    pub name: &'static str,
    pub path: &'static str,
    pub features: &'static str,
}

const DEFUSE_WASM_VAR: &str = "DEFUSE_WASM";
const POA_FACTORY_WASM_VAR: &str = "DEFUSE_POA_FACTORY_WASM";
const POA_TOKEN_WASM_VAR: &str = "DEFUSE_POA_TOKEN_WASM";
const ESCROW_SWAP_WASM_VAR: &str = "DEFUSE_ESCROW_SWAP_WASM";
const MULTI_TOKEN_RECEIVER_STUB_WASM_VAR: &str = "DEFUSE_MULTI_TOKEN_RECEIVER_STUB_WASM";

#[derive(Clone, ValueEnum, Default, Debug)]
pub enum Contract {
    #[default]
    Defuse,
    PoaToken,
    PoaFactory,
    EscrowSwap,
    MultiTokenReceiverStub,
}

impl Contract {
    pub const fn default_env(&self) -> &'static str {
        match self {
            Self::Defuse => DEFUSE_WASM_VAR,
            Self::PoaFactory => POA_FACTORY_WASM_VAR,
            Self::PoaToken => POA_TOKEN_WASM_VAR,
            Self::EscrowSwap => ESCROW_SWAP_WASM_VAR,
            Self::MultiTokenReceiverStub => MULTI_TOKEN_RECEIVER_STUB_WASM_VAR,
        }
    }

    pub const fn spec(&self) -> ContractSpec {
        match self {
            Self::Defuse => ContractSpec {
                name: "defuse",
                path: "defuse",
                features: "contract,imt",
            },
            Self::PoaFactory => ContractSpec {
                name: "poa-factory",
                path: "poa-factory",
                features: "contract",
            },
            Self::PoaToken => ContractSpec {
                name: "poa-token",
                path: "poa-token",
                features: "contract",
            },
            Self::EscrowSwap => ContractSpec {
                name: "escrow-swap",
                path: "escrow-swap",
                features: "contract",
            },
            Self::MultiTokenReceiverStub => ContractSpec {
                name: "multi-token-receiver-stub",
                path: "tests/contracts/multi-token-receiver-stub",
                features: "",
            },
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Defuse,
            Self::PoaToken,
            Self::PoaFactory,
            Self::EscrowSwap,
            Self::MultiTokenReceiverStub,
        ]
    }
}
