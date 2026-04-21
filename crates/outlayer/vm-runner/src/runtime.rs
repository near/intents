use anyhow::Result;
use defuse_outlayer_host_functions::{HostFunctions, Imports};
use std::marker::PhantomData;
use tracing::instrument;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store, StoreLimitsBuilder, Trap};
use wasmtime_wasi::WasiCtx;
use wasmtime_wasi::p2::bindings::Command;
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};

use crate::error::ExecutionError;
use crate::host::HostCtx;
use crate::outcome::ExecutionOutcome;

const MEMORY_GUARD_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB
const DEFAULT_MEMORY_LIMIT: usize = 100 * 1024 * 1024; // 100 MiB
const DEFAULT_FUEL_LIMIT: u64 = 1_000_000_000;

const STDOUT_MAX_SIZE: usize = 4 * 1024 * 1024; // 4 MiB
const STDERR_MAX_SIZE: usize = 64 * 1024; // 64 KiB

/// A builder for configuring and creating a `VmRuntime`
/// with a custom host environment.
pub struct VmRuntimeBuilder<H: HostFunctions + 'static> {
    config: Config,
    fuel_limit: Option<u64>,
    memory_limit: Option<usize>,
    _host: PhantomData<H>,
}

impl<H: HostFunctions + 'static> VmRuntimeBuilder<H> {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.guard_before_linear_memory(true);
        config.memory_guard_size(MEMORY_GUARD_SIZE);
        // NOTE: this is enabling async in host-defined functions
        config.async_support(true);
        config.consume_fuel(true);

        Self {
            config,
            fuel_limit: None,
            memory_limit: None,
            _host: PhantomData,
        }
    }

    /// Sets the fuel limit for the runtime
    /// If not set, a default value will be used
    #[must_use]
    pub const fn fuel_limit(mut self, fuel_limit: u64) -> Self {
        self.fuel_limit = Some(fuel_limit);
        self
    }

    /// Sets the memory limit for the runtime
    /// If not set, a default value will be used
    #[must_use]
    pub const fn memory_limit(mut self, memory_limit: usize) -> Self {
        self.memory_limit = Some(memory_limit);
        self
    }

    /// Builds the `VmRuntime` with the specified configuration
    /// If fuel limit or memory limit are not set, default values will be used
    pub fn build(mut self) -> Result<VmRuntime<H>> {
        let memory_limit = self.memory_limit.unwrap_or(DEFAULT_MEMORY_LIMIT);
        // NOTE: set initial chunk of virtual memory that a linear memory
        // may grow into limited to allow multiple linear memories to be
        // instantiated without exhausting host resources
        self.config
            .memory_reservation(memory_limit.try_into().unwrap());

        let engine = Engine::new(&self.config)?;
        let linker = create_linker::<H>(&engine)?;

        Ok(VmRuntime {
            engine,
            fuel_limit: self.fuel_limit.unwrap_or(DEFAULT_FUEL_LIMIT),
            memory_limit,
            linker,
        })
    }
}

fn create_linker<H: HostFunctions + 'static>(engine: &Engine) -> Result<Linker<HostCtx<H>>> {
    let mut linker = Linker::new(engine);

    // Add WASI imports to the linker
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

    // Add host function imports to the linker
    Imports::add_to_linker::<HostCtx<H>, HasSelf<H>>(&mut linker, |ctx: &mut HostCtx<H>| {
        ctx.host_state_mut()
    })?;

    Ok(linker)
}

impl<H: HostFunctions + 'static> Default for VmRuntimeBuilder<H> {
    fn default() -> Self {
        Self::new()
    }
}

/// A runtime for executing wasip2 components with a custom host
/// environment.
///
/// The host environment is defined by the `HostFunctions` trait,
/// which must be implemented by the user and provided as a type
/// parameter to the builder.
pub struct VmRuntime<H: HostFunctions + 'static> {
    engine: Engine,
    fuel_limit: u64,
    memory_limit: usize,
    linker: Linker<HostCtx<H>>,
}

impl<H: HostFunctions + 'static> VmRuntime<H> {
    /// Compile wasip2 component from the given binary data
    pub fn compile(&self, binary: impl AsRef<[u8]>) -> Result<Component> {
        Component::from_binary(&self.engine, binary.as_ref())
    }

    /// Executes the `run` function of the given component with the
    /// provided host state and input.
    ///
    /// Example:
    /// ```rust, no_run
    /// use defuse_outlayer_state::HostState;
    /// use defuse_outlayer_vm_runner::VmRuntimeBuilder;
    ///
    /// let host_state = HostState::default();
    /// let runner = VmRuntimeBuilder::<HostState>::new().build()?;
    /// let component = runner.compile(&wasm_binary)?;
    ///
    /// runner.execute(&component, host_state, b"Hello").await?;
    /// ```
    #[instrument(skip_all)]
    pub async fn execute(
        &self,
        component: &Component,
        host_state: H,
        input: impl AsRef<[u8]>,
    ) -> Result<ExecutionOutcome, ExecutionError> {
        let stdin = MemoryInputPipe::new(input.as_ref().to_vec());
        let stdout = MemoryOutputPipe::new(STDOUT_MAX_SIZE);
        let stderr = MemoryOutputPipe::new(STDERR_MAX_SIZE);
        let wasi_ctx = WasiCtx::builder()
            .stdin(stdin)
            .stdout(stdout.clone())
            .stderr(stderr.clone())
            .build();

        let limits = StoreLimitsBuilder::new()
            .memory_size(self.memory_limit)
            .build();
        let mut store = Store::new(&self.engine, HostCtx::new(wasi_ctx, host_state, limits));

        store.limiter(|ctx| ctx.limits_mut());
        store
            .set_fuel(self.fuel_limit)
            .expect("Fuel consumption is not enabled on engine");

        let program_result = self.run_command(&mut store, component).await;

        let fuel_consumed = self
            .fuel_limit
            .saturating_sub(store.get_fuel().unwrap_or(self.fuel_limit));

        process_execution_result(fuel_consumed, &stdout, &stderr, program_result)
    }

    async fn run_command(
        &self,
        store: &mut Store<HostCtx<H>>,
        component: &Component,
    ) -> anyhow::Result<()> {
        let command = Command::instantiate_async(&mut *store, component, &self.linker).await?;

        command
            .wasi_cli_run()
            .call_run(&mut *store)
            .await?
            .map_err(|()| anyhow::anyhow!("component run() returned Err(())"))
    }
}

fn process_execution_result(
    fuel_consumed: u64,
    stdout: &MemoryOutputPipe,
    stderr: &MemoryOutputPipe,
    program_result: anyhow::Result<()>,
) -> Result<ExecutionOutcome, ExecutionError> {
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
            Err(err)
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
