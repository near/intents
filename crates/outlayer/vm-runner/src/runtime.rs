use anyhow::Result;
use defuse_outlayer_host_functions::{HostFunctions, Imports};
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

pub struct VmRuntimeBuilder<B: WasiBackend, H: HostFunctions + 'static> {
    config: Config,
    fuel_limit: Option<u64>,
    memory_limit: Option<usize>,
    _backend: PhantomData<B>,
    _host: PhantomData<H>,
}

impl<B: WasiBackend, H: HostFunctions + 'static> VmRuntimeBuilder<B, H> {
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
            _host: PhantomData,
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

    pub fn build(mut self) -> Result<VmRuntime<B, H>> {
        let memory_limit = self.memory_limit.unwrap_or(DEFAULT_MEMORY_LIMIT);
        self.config
            .memory_reservation(memory_limit.try_into().unwrap());

        let engine = Engine::new(&self.config)?;
        let linker = create_linker::<B, H>(&engine)?;

        Ok(VmRuntime {
            engine,
            fuel_limit: self.fuel_limit.unwrap_or(DEFAULT_FUEL_LIMIT),
            memory_limit,
            linker,
        })
    }
}

fn create_linker<B: WasiBackend, H: HostFunctions + 'static>(
    engine: &Engine,
) -> Result<Linker<HostCtx<B::State, H>>> {
    let mut linker = Linker::new(engine);

    B::setup_linker(&mut linker)?;

    Imports::add_to_linker::<HostCtx<B::State, H>, HasSelf<H>>(
        &mut linker,
        |ctx: &mut HostCtx<B::State, H>| ctx.host_state_mut(),
    )?;

    Ok(linker)
}

impl<B: WasiBackend, H: HostFunctions + 'static> Default for VmRuntimeBuilder<B, H> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct VmRuntime<B: WasiBackend, H: HostFunctions + 'static> {
    engine: Engine,
    fuel_limit: u64,
    memory_limit: usize,
    linker: Linker<HostCtx<B::State, H>>,
}

impl<B: WasiBackend, H: HostFunctions + 'static> VmRuntime<B, H> {
    pub fn load(&self, binary: impl AsRef<[u8]>) -> Result<Component> {
        Component::from_binary(&self.engine, binary.as_ref())
    }

    /// Executes the `run` function of the given component with the
    /// provided host state and input.
    ///
    /// Example:
    /// ```rust
    /// use defuse_outlayer_state::HostState;
    /// use defuse_outlayer_vm_runner::{VmRuntimeBuilder, WasiP2Backend};
    ///
    /// let host_state = HostState::default();
    /// let runner = VmRuntimeBuilder::<WasiP2Backend, HostState>::new().build()?;
    /// let component = runner.load(&wasm_binary)?;
    ///
    /// runner.execute(&component, host_state, "Hello").await?;
    /// ```
    #[instrument(skip_all)]
    pub async fn execute<I>(
        &self,
        component: &Component,
        host_state: H,
        input: I,
    ) -> Result<ExecutionOutcome, VmError>
    where
        I: serde::Serialize + Send,
    {
        let input_bytes = serde_json::to_vec(&input).map_err(VmError::InvalidInput)?;
        let stdin = MemoryInputPipe::new(input_bytes);
        let stdout = MemoryOutputPipe::new(STDOUT_MAX_SIZE);
        let stderr = MemoryOutputPipe::new(STDERR_MAX_SIZE);
        let wasi_state = B::build_state(stdin, stdout.clone(), stderr.clone());

        let limits = StoreLimitsBuilder::new()
            .memory_size(self.memory_limit)
            .build();
        let mut store = Store::new(&self.engine, HostCtx::new(wasi_state, host_state, limits));

        store.limiter(|ctx| ctx.limits_mut());
        store
            .set_fuel(self.fuel_limit)
            .expect("Fuel consumption is not enabled on engine");

        let program_result = B::run(&mut store, component, &self.linker).await;

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
    let trap = match program_result {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };

    let stderr = String::from_utf8_lossy(&stderr.contents()).into_owned();

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

    Err(match trap_code {
        Some(code) => ExecutionError::Trap { code, stderr },
        None => ExecutionError::Unknown {
            source: trap,
            stderr,
        },
    })
}
