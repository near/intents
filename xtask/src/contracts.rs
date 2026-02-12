use clap::ValueEnum;

pub struct ContractSpec {
    pub name: &'static str,
    pub path: &'static str,
    pub features: &'static str,
}

#[derive(Clone, ValueEnum, Default, Debug)]
pub enum Contract {
    #[default]
    Defuse,
    PoaToken,
    PoaFactory,
    EscrowSwap,
    MultiTokenReceiverStub,
    Deployer,
}

impl Contract {
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
            Self::Deployer => ContractSpec {
                name: "global-deployer",
                path: "global-deployer",
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
            Self::Deployer,
        ]
    }
}
