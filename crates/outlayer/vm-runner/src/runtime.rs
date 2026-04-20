use anyhow::Result;
use defuse_outlayer_sys::host::{Host, outlayer};
use std::marker::PhantomData;
use tracing::instrument;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store, Trap};
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};

use crate::backend::WasiBackend;
use crate::error::{ExecutionError, VmError};
use crate::host::HostCtx;
use crate::outcome::ExecutionOutcome;

const MEMORY_GUARD_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB
const MEMORY_RESERVATION_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB
const DEFAULT_FUEL_LIMIT: u64 = 1_000_000_000;

const STDOUT_MAX_SIZE: usize = 4 * 1024 * 1024; // 4 MiB
const STDERR_MAX_SIZE: usize = 64 * 1024; // 64 KiB

pub struct VmRuntimeBuilder<W: WasiBackend> {
    config: Config,
    fuel_limit: Option<u64>,
    _backend: PhantomData<W>,
}

impl<W: WasiBackend> VmRuntimeBuilder<W> {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.memory_reservation(MEMORY_RESERVATION_SIZE);
        config.guard_before_linear_memory(true);
        config.memory_guard_size(MEMORY_GUARD_SIZE);

        // NOTE: this is required for async host functions
        config.async_support(true);

        config.consume_fuel(true);

        Self {
            config,
            fuel_limit: None,
            _backend: PhantomData,
        }
    }

    #[must_use]
    pub const fn fuel_limit(mut self, fuel_limit: u64) -> Self {
        self.fuel_limit = Some(fuel_limit);
        self
    }

    pub fn build(&self) -> Result<VmRuntime<W>> {
        Ok(VmRuntime {
            engine: Engine::new(&self.config)?,
            fuel_limit: self.fuel_limit.unwrap_or(DEFAULT_FUEL_LIMIT),
            _backend: PhantomData,
        })
    }
}

impl<W: WasiBackend> Default for VmRuntimeBuilder<W> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct VmRuntime<W: WasiBackend> {
    engine: Engine,
    fuel_limit: u64,
    _backend: PhantomData<W>,
}

impl<W: WasiBackend> VmRuntime<W> {
    fn create_linker<H: Host + 'static>(&self) -> Result<Linker<HostCtx<W::State, H>>> {
        let mut linker = Linker::new(&self.engine);

        W::setup_linker(&mut linker)?;

        outlayer::crypto::ed25519::add_to_linker::<HostCtx<W::State, H>, HasSelf<H::Ed25519>>(
            &mut linker,
            |ctx: &mut HostCtx<W::State, H>| ctx.host_state.ed25519(),
        )?;

        outlayer::crypto::secp256k1::add_to_linker::<HostCtx<W::State, H>, HasSelf<H::Secp256k1>>(
            &mut linker,
            |ctx: &mut HostCtx<W::State, H>| ctx.host_state.secp256k1(),
        )?;

        Ok(linker)
    }

    pub fn load(&self, binary: impl AsRef<[u8]>) -> Result<Component> {
        Component::from_binary(&self.engine, binary.as_ref())
    }

    #[instrument(skip_all)]
    pub async fn execute<H, I>(
        &self,
        component: &Component,
        host_state: H,
        input: I,
    ) -> Result<ExecutionOutcome, VmError>
    where
        H: Host + 'static,
        I: serde::Serialize,
    {
        let linker = self.create_linker::<H>()?;

        let input_bytes = serde_json::to_vec(&input).map_err(ExecutionError::InvalidInput)?;
        let stdin = MemoryInputPipe::new(input_bytes);
        let stdout = MemoryOutputPipe::new(STDOUT_MAX_SIZE);
        let stderr = MemoryOutputPipe::new(STDERR_MAX_SIZE);
        let wasi_state = W::build_state(stdin, stdout.clone(), stderr.clone());

        let mut store = Store::new(&self.engine, HostCtx::new(wasi_state, host_state));
        store
            .set_fuel(self.fuel_limit)
            .expect("Fuel consumption is not enabled on engine");

        let program_result = W::call_run(&mut store, component, &linker).await;

        let fuel_consumed = self
            .fuel_limit
            .saturating_sub(store.get_fuel().unwrap_or(0));

        process_execution_result(fuel_consumed, &stdout, &stderr, program_result)
    }
}

fn process_execution_result(
    fuel_consumed: u64,
    stdout: &MemoryOutputPipe,
    stderr: &MemoryOutputPipe,
    program_result: anyhow::Result<()>,
) -> Result<ExecutionOutcome, VmError> {
    let stderr = String::from_utf8_lossy(&stderr.contents()).into_owned();

    match program_result {
        Ok(()) => {
            tracing::debug!(fuel_consumed, "execution succeeded");
            Ok(ExecutionOutcome {
                output: stdout.contents().to_vec(),
                stderr,
                fuel_consumed,
            })
        }
        Err(trap) => {
            let err = if let Some(exit) = trap.downcast_ref::<wasmtime_wasi::I32Exit>() {
                ExecutionError::NonZeroExit {
                    code: exit.0,
                    stderr,
                }
            } else {
                let trap_code = trap.chain().find_map(|e| e.downcast_ref::<Trap>().copied());
                match trap_code {
                    Some(code) => ExecutionError::Trap { code, stderr },
                    None => ExecutionError::Unknown {
                        source: trap,
                        stderr,
                    },
                }
            };

            tracing::debug!(error = ?err, fuel_consumed, "execution failed");

            Err(err.into())
        }
    }
}
