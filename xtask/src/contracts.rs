use clap::ValueEnum;

pub struct ContractSpec {
    pub name: &'static str,
    pub path: &'static str,
    pub features: &'static str,
    pub env_var_key: &'static str,
}

#[derive(Clone, ValueEnum, Default, Debug)]
pub enum Contract {
    #[default]
    Defuse,
    PoaFactory,
    PoaToken,
    EscrowSwap,
    MultiTokenReceiverStub,
}

impl Contract {
    pub const fn spec(&self) -> ContractSpec {
        match self {
            Self::Defuse => ContractSpec {
                name: "defuse",
                path: "defuse",
                features: "contract,imt",
                env_var_key: "DEFUSE_WASM",
            },
            Self::PoaFactory => ContractSpec {
                name: "poa-factory",
                path: "poa-factory",
                features: "contract",
                env_var_key: "DEFUSE_POA_FACTORY_WASM",
            },
            Self::PoaToken => ContractSpec {
                name: "poa-token",
                path: "poa-token",
                features: "contract",
                env_var_key: "DEFUSE_POA_TOKEN_WASM",
            },
            Self::EscrowSwap => ContractSpec {
                name: "escrow-swap",
                path: "escrow-swap",
                features: "contract",
                env_var_key: "DEFUSE_ESCROW_SWAP_WASM",
            },
            Self::MultiTokenReceiverStub => ContractSpec {
                name: "multi-token-receiver-stub",
                path: "tests/contracts/multi-token-receiver-stub",
                features: "contract",
                env_var_key: "MULTI_TOKEN_RECEIVER_STUB_WASM",
            },
        }
    }

    pub const fn all() -> &'static [Contract] {
        &[
            Self::Defuse,
            Self::PoaFactory,
            Self::PoaToken,
            Self::EscrowSwap,
            Self::MultiTokenReceiverStub,
        ]
    }
}
