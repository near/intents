use anyhow::{Context as _, Result, anyhow};
use defuse_outlayer_host::{HostFunctions, bindings::Imports};
use tracing::instrument;
use wasmtime::{
    Config, Engine, Store, StoreLimitsBuilder, Trap,
    component::{Component, HasSelf, Linker},
};
use wasmtime_wasi::{
    WasiCtx,
    cli::{StdinStream, StdoutStream},
    p2::bindings::Command,
};

use crate::error::ExecutionError;
use crate::outcome::ExecutionOutcome;
use crate::{context::HostCtx, outcome::ExecutionDetails};

/// Size of the guard region placed before linear memory to
/// catch out-of-bounds accesses
const MEMORY_GUARD_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB

pub struct Context<I, O, E, H> {
    input: I,
    output: O,
    error: E,
    host_state: H,
    fuel_limit: Option<u64>,
    memory_limit: Option<usize>,
}

impl<I, O, E, H> Context<I, O, E, H>
where
    I: StdinStream + 'static,
    O: StdoutStream + 'static,
    E: StdoutStream + 'static,
    H: HostFunctions,
{
    /// Default maximum physical memory for a single component
    /// execution (100 MiB)
    pub const DEFAULT_MEMORY_LIMIT: usize = 100 * 1024 * 1024;

    /// Default fuel budget for a single component execution
    /// (~1 billion wasm instructions)
    pub const DEFAULT_FUEL_LIMIT: u64 = 1_000_000_000;

    /// Creates a new execution context
    /// By default, there are no fuel or memory limits
    #[must_use]
    pub const fn new(input: I, output: O, error: E, host_state: H) -> Self {
        Self {
            input,
            output,
            error,
            host_state,
            fuel_limit: None,
            memory_limit: None,
        }
    }

    /// Sets the maximum fuel the component may consume per execution
    ///
    /// Fuel roughly corresponds to the number of WebAssembly instructions
    /// executed. Exceeding the limit raises [`ExecutionError::Trap`].
    #[must_use]
    pub const fn fuel_limit(mut self, fuel_limit: u64) -> Self {
        self.fuel_limit = Some(fuel_limit);
        self
    }

    /// Sets the maximum physical memory the components linear memory may use
    ///
    /// Attempts to grow beyond this limit will trap. Defaults to
    #[must_use]
    pub const fn memory_limit(mut self, memory_limit: usize) -> Self {
        self.memory_limit = Some(memory_limit);
        self
    }

    pub fn into_store(self, engine: &Engine) -> Store<HostCtx<H>> {
        let Self {
            input,
            output,
            error,
            host_state,
            fuel_limit,
            memory_limit,
        } = self;

        let wasi_ctx = WasiCtx::builder()
            .stdin(input)
            .stdout(output)
            .stderr(error)
            .build();

        let limits = memory_limit.map_or_else(StoreLimitsBuilder::new, |m| {
            StoreLimitsBuilder::new().memory_size(m)
        });

        let mut store = Store::new(engine, HostCtx::new(wasi_ctx, host_state, limits.build()));

        store.limiter(|ctx| ctx.limits_mut());

        if let Some(fuel_limit) = fuel_limit {
            store.set_fuel(fuel_limit).expect("fuel must be enabled");
        }

        store
    }
}

/// A runtime for executing wasip2 components with a custom host
/// environment.
///
/// The host environment is defined by the `HostFunctions` trait,
/// which must be implemented by the user and provided as a type
/// parameter to the builder.
pub struct VmRuntime<H: HostFunctions + 'static> {
    linker: Linker<HostCtx<H>>,
}

impl<H: HostFunctions + 'static> VmRuntime<H> {
    /// Creates a new `VmRuntime` with default configuration.
    ///
    /// Async support and fuel metering are always enabled and cannot be
    /// disabled.
    pub fn new() -> anyhow::Result<Self> {
        let mut config = Config::new();
        config.guard_before_linear_memory(true);
        config.memory_guard_size(MEMORY_GUARD_SIZE);
        // NOTE: this is enabling async in host-defined functions
        config.async_support(true);
        config.consume_fuel(true);

        // TODO: uncomment in corresponding pr
        // // NOTE: set initial chunk of virtual memory that a linear memory
        // // may grow into limited to allow multiple linear memories to be
        // // instantiated without exhausting host resources
        // self.config
        //     .memory_reservation(self.memory_limit.try_into().unwrap());

        let engine = Engine::new(&config)?;
        let linker = create_linker::<H>(&engine)?;

        Ok(Self { linker })
    }

    /// Compile wasip2 component from the given binary data
    pub fn compile(&self, binary: impl AsRef<[u8]>) -> Result<Component> {
        Component::from_binary(self.linker.engine(), binary.as_ref())
    }

    /// Executes the `wasi:cli/run` function of the given component.
    ///
    /// stdin, stdout, stderr, and the host state are provided via [`Context`].
    /// The caller is responsible for reading output from the stdout/stderr
    /// streams after execution (e.g. by keeping a clone of [`MemoryOutputPipe`]
    /// before passing it into the context)
    ///
    /// Execution is bounded by the fuel and memory limits configured on the
    /// [`Context`]. Fuel exhaustion is reported as [`ExecutionError::Trap`]
    ///
    /// # Example
    ///
    /// Using [`State`] directly:
    ///
    /// ```rust,no_run
    /// use std::borrow::Cow;
    /// use defuse_outlayer_host::{State, host::Context as HostContext};
    /// use defuse_outlayer_vm_runner::{Context, VmRuntime};
    /// use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let app_id = todo!();
    /// # let signer = todo!();
    /// let state = State::new(HostContext { app_id }, Cow::Owned(signer));
    ///
    /// let stdout = MemoryOutputPipe::new(4 * 1024 * 1024);
    /// let stderr = MemoryOutputPipe::new(64 * 1024);
    /// let ctx = Context::new(
    ///     MemoryInputPipe::new(b"input".to_vec()),
    ///     stdout.clone(),
    ///     stderr.clone(),
    ///     state,
    /// );
    ///
    /// let runner = VmRuntime::<State>::new()?;
    /// let wasm = std::fs::read("component.wasm")?;
    /// let component = runner.compile(&wasm)?;
    /// runner.execute(ctx, &component).await?;
    ///
    /// let stdout = stdout.contents();
    /// let stderr = stderr.contents();
    ///
    /// # Ok(()) }
    /// ```
    #[instrument(skip_all)]
    pub async fn execute<I, O, E>(
        &self,
        ctx: Context<I, O, E, H>,
        component: &Component,
    ) -> Result<ExecutionOutcome>
    where
        I: StdinStream + 'static,
        O: StdoutStream + 'static,
        E: StdoutStream + 'static,
    {
        let fuel_limit = ctx.fuel_limit;
        let mut store = ctx.into_store(self.linker.engine());

        let command = Command::instantiate_async(&mut store, component, &self.linker).await?;
        let run_result = command.wasi_cli_run().call_run(&mut store).await;

        let error = match run_result {
            Ok(Ok(())) => None,
            Ok(Err(())) => Some(anyhow!("wasm component failed").into()),
            Err(trap) => classify_error(trap),
        };

        let fuel_consumed = fuel_limit
            .map(|limit| limit.saturating_sub(store.get_fuel().expect("fuel must be enabled")));

        Ok(ExecutionOutcome::new(
            ExecutionDetails::new(fuel_consumed),
            error,
        ))
    }

    /// Convenience method to compile and execute a component in one step. See
    /// [`execute`](Self::execute) for details and examples.
    pub async fn run<I, O, E>(
        &self,
        ctx: Context<I, O, E, H>,
        binary: impl AsRef<[u8]>,
    ) -> Result<ExecutionOutcome>
    where
        I: StdinStream + 'static,
        O: StdoutStream + 'static,
        E: StdoutStream + 'static,
    {
        let component = self.compile(binary).context("compile")?;
        self.execute(ctx, &component).await
    }
}

impl<H> Default for VmRuntime<H>
where
    H: HostFunctions,
{
    fn default() -> Self {
        Self::new().expect("setup")
    }
}

fn classify_error(err: anyhow::Error) -> Option<ExecutionError> {
    if let Some(exit) = err.downcast_ref::<wasmtime_wasi::I32Exit>() {
        tracing::debug!("Program exited with code {}", exit.0);

        // exit(0) is equivalent to a clean return from main
        return match exit.0 {
            0 => None,
            code => Some(ExecutionError::NonZeroExit(code)),
        };
    }

    Some(
        err.downcast_ref::<Trap>()
            .copied()
            .map_or_else(|| ExecutionError::Unknown(err), ExecutionError::Trap),
    )
}

fn create_linker<H>(engine: &Engine) -> anyhow::Result<Linker<HostCtx<H>>>
where
    H: HostFunctions,
{
    let mut linker = Linker::new(engine);

    // Add WASI imports to the linker
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

    // Add host function imports to the linker
    Imports::add_to_linker::<HostCtx<H>, HasSelf<H>>(&mut linker, |ctx: &mut HostCtx<H>| {
        ctx.host_state_mut()
    })?;

    Ok(linker)
}
