use anyhow::Result;

use defuse_outlayer_sys::host::{Host, outlayer};

use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store, Trap};
use wasmtime_wasi::p2::bindings::Command;
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
use wasmtime_wasi::WasiCtx;

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

    pub async fn execute<T, I>(
        &self,
        wasm_path: &str,
        host_state: T,
        input: I,
    ) -> Result<ExecutionOutcome>
    where
        T: Host + 'static,
        I: serde::Serialize,
    {
        let linker = self.create_linker::<T>()?;

        let stdin = MemoryInputPipe::new(serde_json::to_vec(&input)?);
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

        let component = Component::from_file(&self.engine, wasm_path)?;
        let command = Command::instantiate_async(&mut store, &component, &linker).await?;
        let program_result = command.wasi_cli_run().call_run(&mut store).await;

        let fuel_consumed = store.get_fuel().unwrap_or(0);
        let stderr_output = stderr.contents();
        let stderr_str = String::from_utf8_lossy(&stderr_output);

        match program_result {
            Ok(_) => Ok(ExecutionOutcome {
                output: stdout.contents().to_vec(),
                stderr: stderr_str.into_owned(),
                fuel_consumed,
            }),
            Err(trap) => {
                if let Some(exit) = trap.downcast_ref::<wasmtime_wasi::I32Exit>() {
                    anyhow::bail!("{stderr_str}\nComponent exited with code {}", exit.0);
                }
                match trap.downcast_ref::<Trap>() {
                    Some(Trap::UnreachableCodeReached) => {
                        anyhow::bail!("{stderr_str}\nComponent panicked")
                    }
                    Some(Trap::OutOfFuel) => anyhow::bail!("fuel exhausted"),
                    Some(t) => anyhow::bail!("{stderr_str}\nTrap: {t}"),
                    None => anyhow::bail!("{stderr_str}\nComponent trapped: {trap}"),
                }
            }
        }
    }
}
