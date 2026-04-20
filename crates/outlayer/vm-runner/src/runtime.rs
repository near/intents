use anyhow::Result;
use defuse_outlayer_sys::host::{Host, outlayer};
use std::marker::PhantomData;
use tracing::instrument;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store, StoreLimitsBuilder, Trap};
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};

use crate::backend::WasiBackend;
use crate::error::{ExecutionError, VmError};
use crate::host::HostCtx;
use crate::outcome::ExecutionOutcome;

const MEMORY_GUARD_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB
const DEFAULT_MEMORY_LIMIT: usize = 100 * 1024 * 1024; // 100 MiB
const DEFAULT_FUEL_LIMIT: u64 = 1_000_000_000;

const STDOUT_MAX_SIZE: usize = 4 * 1024 * 1024; // 4 MiB
const STDERR_MAX_SIZE: usize = 64 * 1024; // 64 KiB

pub struct VmRuntimeBuilder<B: WasiBackend> {
    config: Config,
    fuel_limit: Option<u64>,
    memory_limit: Option<usize>,
    _backend: PhantomData<B>,
}

impl<B: WasiBackend> VmRuntimeBuilder<B> {
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
            memory_limit: None,
            _backend: PhantomData,
        }
    }

    #[must_use]
    pub const fn fuel_limit(mut self, fuel_limit: u64) -> Self {
        self.fuel_limit = Some(fuel_limit);
        self
    }

    #[must_use]
    pub const fn memory_limit(mut self, memory_limit: usize) -> Self {
        self.memory_limit = Some(memory_limit);
        self
    }

    pub fn build(mut self) -> Result<VmRuntime<B>> {
        let memory_limit = self.memory_limit.unwrap_or(DEFAULT_MEMORY_LIMIT);
        self.config
            .memory_reservation(memory_limit.try_into().unwrap());

        Ok(VmRuntime {
            engine: Engine::new(&self.config)?,
            fuel_limit: self.fuel_limit.unwrap_or(DEFAULT_FUEL_LIMIT),
            memory_limit,
            _backend: PhantomData,
        })
    }
}

impl<B: WasiBackend> Default for VmRuntimeBuilder<B> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct VmRuntime<B: WasiBackend> {
    engine: Engine,
    fuel_limit: u64,
    memory_limit: usize,
    _backend: PhantomData<B>,
}

impl<B: WasiBackend> VmRuntime<B> {
    fn create_linker<H: Host + 'static>(&self) -> Result<Linker<HostCtx<B::State, H>>> {
        let mut linker = Linker::new(&self.engine);

        B::setup_linker(&mut linker)?;

        outlayer::crypto::ed25519::add_to_linker::<HostCtx<B::State, H>, HasSelf<H::Ed25519>>(
            &mut linker,
            |ctx: &mut HostCtx<B::State, H>| ctx.host_state.ed25519(),
        )?;

        outlayer::crypto::secp256k1::add_to_linker::<HostCtx<B::State, H>, HasSelf<H::Secp256k1>>(
            &mut linker,
            |ctx: &mut HostCtx<B::State, H>| ctx.host_state.secp256k1(),
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
        I: serde::Serialize + Send,
    {
        let linker = self.create_linker::<H>()?;

        let input_bytes = serde_json::to_vec(&input).map_err(VmError::InvalidInput)?;
        let stdin = MemoryInputPipe::new(input_bytes);
        let stdout = MemoryOutputPipe::new(STDOUT_MAX_SIZE);
        let stderr = MemoryOutputPipe::new(STDERR_MAX_SIZE);
        let wasi_state = B::build_state(stdin, stdout.clone(), stderr.clone());

        let limits = StoreLimitsBuilder::new()
            .memory_size(self.memory_limit)
            .build();
        let mut store = Store::new(&self.engine, HostCtx::new(wasi_state, host_state, limits));

        store.limiter(|ctx| &mut ctx.limits);
        store
            .set_fuel(self.fuel_limit)
            .expect("Fuel consumption is not enabled on engine");

        let program_result = B::call_run(&mut store, component, &linker).await;

        let fuel_consumed = self
            .fuel_limit
            .saturating_sub(store.get_fuel().unwrap_or(self.fuel_limit));

        process_execution_result(fuel_consumed, &stdout, &stderr, program_result)
    }
}

fn process_execution_result(
    fuel_consumed: u64,
    stdout: &MemoryOutputPipe,
    stderr: &MemoryOutputPipe,
    program_result: anyhow::Result<()>,
) -> Result<ExecutionOutcome, VmError> {
    match classify_result(program_result, stderr) {
        Ok(()) => {
            tracing::debug!(fuel_consumed, "execution succeeded");
            Ok(ExecutionOutcome {
                output: stdout.contents().to_vec(),
                stderr: stderr.contents().to_vec(),
                fuel_consumed,
            })
        }
        Err(err) => {
            tracing::debug!(error = ?err, fuel_consumed, "execution failed");
            Err(err.into())
        }
    }
}

fn classify_result(
    program_result: anyhow::Result<()>,
    stderr: &MemoryOutputPipe,
) -> Result<(), ExecutionError> {
    let stderr = String::from_utf8_lossy(&stderr.contents()).into_owned();

    let trap = match program_result {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };

    if let Some(exit) = trap.downcast_ref::<wasmtime_wasi::I32Exit>() {
        // exit(0) is equivalent to a clean return from main
        return if exit.0 == 0 {
            Ok(())
        } else {
            Err(ExecutionError::NonZeroExit {
                code: exit.0,
                stderr,
            })
        };
    }

    let trap_code = trap.chain().find_map(|e| e.downcast_ref::<Trap>().copied());

    Err(trap_code.map_or_else(
        || ExecutionError::Unknown {
            source: trap,
            stderr: stderr.clone(),
        },
        |code| ExecutionError::Trap {
            code,
            stderr: stderr.clone(),
        },
    ))
}
