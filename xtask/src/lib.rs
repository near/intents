use anyhow::Result;
use cargo_near_build::{
    BuildOpts, build_with_cli,
    camino::Utf8PathBuf,
    docker::{DockerBuildOpts, build as build_reproducible_with_cli},
};
use std::{env, marker::PhantomData};

const DEFAULT_OUT_DIR: &str = "../res";
const BUILD_REPRODUCIBLE_ENV_VAR: &str = "DEFUSE_BUILD_REPRODUCIBLE";

pub struct DefuseContract;

impl ContractOptions for DefuseContract {
    fn path() -> &'static str {
        "defuse"
    }

    fn features() -> &'static str {
        "contract,imt"
    }

    fn env_var_key() -> &'static str {
        "DEFUSE_WASM"
    }
}

pub struct PoaFactoryContract;

impl ContractOptions for PoaFactoryContract {
    fn path() -> &'static str {
        "poa-factory"
    }

    fn features() -> &'static str {
        "contract"
    }

    fn env_var_key() -> &'static str {
        "DEFUSE_POA_FACTORY_WASM"
    }
}

pub struct EscrowSwapContract;

impl ContractOptions for EscrowSwapContract {
    fn path() -> &'static str {
        "escrow-swap"
    }

    fn features() -> &'static str {
        "contract"
    }

    fn env_var_key() -> &'static str {
        "DEFUSE_ESCROW_SWAP_WASM"
    }
}

pub struct MultiTokenReceiverStubContract;

impl ContractOptions for MultiTokenReceiverStubContract {
    fn path() -> &'static str {
        "tests/contracts/multi-token-receiver-stub"
    }

    fn features() -> &'static str {
        ""
    }

    fn env_var_key() -> &'static str {
        "MULTI_TOKEN_RECEIVER_STUB_WASM"
    }
}

pub struct PoaTokenContract;

impl ContractOptions for PoaTokenContract {
    fn path() -> &'static str {
        "poa-token"
    }

    fn features() -> &'static str {
        "contract"
    }

    fn env_var_key() -> &'static str {
        "DEFUSE_POA_TOKEN_WASM"
    }
}

pub trait ContractOptions {
    fn path() -> &'static str;
    fn features() -> &'static str;
    fn env_var_key() -> &'static str;
}
pub struct ContractBuilder<T: ContractOptions> {
    features: Option<&'static str>,
    env_var_key: Option<&'static str>,
    contract: PhantomData<T>,
}

impl<T: ContractOptions> ContractBuilder<T> {
    fn builder() -> Self {
        Self {
            features: None,
            env_var_key: None,
            contract: PhantomData::<T>,
        }
    }

    fn set_features(mut self, features: &'static str) -> Self {
        self.features = Some(features);
        self
    }
    fn env_var_key(mut self, env_var_key: &'static str) -> Self {
        self.env_var_key = Some(env_var_key);
        self
    }

    fn build_contract(self, outdir: impl Into<Utf8PathBuf>, reproducible: bool) -> Result<()> {
        let workdir = env::var("CARGO_MANIFEST_DIR")?;
        let manifest = format!("{}/../{}/Cargo.toml", workdir, T::path());

        let features = self.features.unwrap_or(T::features());
        let var = self.env_var_key.unwrap_or(T::env_var_key());

        let path = if reproducible {
            let build_opts = DockerBuildOpts::builder()
                .manifest_path(manifest.into())
                .out_dir(outdir.into())
                .build();

            build_reproducible_with_cli(build_opts, false)?.path
        } else {
            let build_opts = BuildOpts::builder()
                .manifest_path(manifest)
                .features(features)
                .out_dir(outdir)
                .no_abi(true)
                .build();

            build_with_cli(build_opts)?
        };

        // unsafe {
        //     std::env::set_var(var, path);
        // }

        Ok(())
    }
}

pub fn build_contract<T: ContractOptions>(
    build_reproducible: &Option<bool>,
    outdir: &Option<String>,
) -> Result<()> {
    let build_reproducible = build_reproducible.unwrap_or(
        env::var(BUILD_REPRODUCIBLE_ENV_VAR)
            .is_ok_and(|v| !["0", "false"].contains(&v.to_lowercase().as_str())),
    );

    let outdir = outdir.as_deref().unwrap_or(DEFAULT_OUT_DIR);

    println!(
        "Building contracts with reproducible-wasm: {}",
        build_reproducible
    );

    ContractBuilder::<T>::builder().build_contract(outdir, build_reproducible)
}

pub fn build_workspace_contracts(
    build_reproducible: &Option<bool>,
    outdir: &Option<String>,
) -> Result<()> {
    build_contract::<DefuseContract>(build_reproducible, outdir)?;
    build_contract::<PoaFactoryContract>(build_reproducible, outdir)?;
    build_contract::<PoaTokenContract>(build_reproducible, outdir)?;
    build_contract::<EscrowSwapContract>(build_reproducible, outdir)?;
    build_contract::<MultiTokenReceiverStubContract>(build_reproducible, outdir)?;

    Ok(())
}
