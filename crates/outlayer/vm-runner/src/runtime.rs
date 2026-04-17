use anyhow::Result;
use defuse_outlayer_sys::host::{Host, outlayer};
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store, Trap};
use wasmtime_wasi::WasiCtx;
use wasmtime_wasi::p2::bindings::Command;
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};

use crate::error::{ExecutionError, VmError};
use crate::host::{HostCtx, WasiP2State};
use crate::outcome::ExecutionOutcome;

const MEMORY_GUARD_SIZE: u64 = 64 * 1024 * 1024;
const DEFAULT_FUEL_LIMIT: u64 = 10_000;
const STDOUT_MAX_SIZE: usize = 4 * 1024 * 1024;
const STDERR_MAX_SIZE: usize = 64 * 1024;

pub struct VmRuntimeBuilder {
    config: Config,
    fuel_limit: Option<u64>,
}

impl VmRuntimeBuilder {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.guard_before_linear_memory(true);
        config.memory_guard_size(MEMORY_GUARD_SIZE);
        // NOTE: this is required for async host functions
        config.async_support(true);
        config.consume_fuel(true);

        Self {
            config,
            fuel_limit: None,
        }
    }

    pub fn fuel_limit(mut self, fuel_limit: u64) -> Self {
        self.fuel_limit = Some(fuel_limit);
        self
    }

    pub fn build(&self) -> Result<VmRuntime> {
        Ok(VmRuntime {
            engine: Engine::new(&self.config)?,
            fuel_limit: self.fuel_limit.unwrap_or(DEFAULT_FUEL_LIMIT),
        })
    }
}

impl Default for VmRuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct VmRuntime {
    engine: Engine,
    fuel_limit: u64,
}

impl VmRuntime {
    fn create_linker<T: Host>(&self) -> Result<Linker<HostCtx<T>>> {
        let mut linker = Linker::new(&self.engine);

        // NOTE: currently only wasip2 is supported
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

        outlayer::crypto::ed25519::add_to_linker::<HostCtx<T>, HasSelf<T::Ed25519>>(
            &mut linker,
            |state: &mut HostCtx<T>| state.host_state.ed25519(),
        )?;

        outlayer::crypto::secp256k1::add_to_linker::<HostCtx<T>, HasSelf<T::Secp256k1>>(
            &mut linker,
            |state: &mut HostCtx<T>| state.host_state.secp256k1(),
        )?;

        Ok(linker)
    }

    pub fn load(&self, wasm_path: &str) -> Result<Component> {
        Component::from_file(&self.engine, wasm_path)
    }

    pub async fn execute<T, I>(
        &self,
        component: &Component,
        host_state: T,
        input: I,
    ) -> Result<ExecutionOutcome, VmError>
    where
        T: Host + 'static,
        I: serde::Serialize,
    {
        let linker = self.create_linker::<T>()?;

        let stdin =
            MemoryInputPipe::new(serde_json::to_vec(&input).map_err(ExecutionError::InvalidInput)?);
        let stdout = MemoryOutputPipe::new(STDOUT_MAX_SIZE);
        let stderr = MemoryOutputPipe::new(STDERR_MAX_SIZE);

        let wasi_ctx = WasiP2State::new(
            WasiCtx::builder()
                .stdin(stdin)
                .stdout(stdout.clone())
                .stderr(stderr.clone())
                .build(),
        );
        let ctx = HostCtx::new(wasi_ctx, host_state);

        let mut store = Store::new(&self.engine, ctx);
        store.set_fuel(self.fuel_limit)?;

        let command = Command::instantiate_async(&mut store, component, &linker).await?;

        let program_result = command.wasi_cli_run().call_run(&mut store).await;

        let fuel_consumed = self
            .fuel_limit
            .saturating_sub(store.get_fuel().unwrap_or(0));

        let stderr = String::from_utf8_lossy(&stderr.contents()).into_owned();

        match program_result {
            Ok(_) => Ok(ExecutionOutcome {
                output: stdout.contents().to_vec(),
                stderr,
                fuel_consumed,
            }),
            Err(trap) => {
                let err = if let Some(exit) = trap.downcast_ref::<wasmtime_wasi::I32Exit>() {
                    ExecutionError::NonZeroExit {
                        code: exit.0,
                        stderr,
                    }
                } else {
                    let trap_code = trap.downcast_ref::<Trap>();
                    match trap_code {
                        Some(code) => ExecutionError::Trap {
                            code: *code,
                            stderr,
                        },
                        None => ExecutionError::Unknown {
                            source: trap,
                            stderr,
                        },
                    }
                };
                Err(err.into())
            }
        }
    }
}
