use anyhow::{Context as _, Result, anyhow};
use defuse_outlayer_host::bindings::Imports;
use tracing::instrument;
use wasmtime::{
    Config, Engine, Store, StoreLimitsBuilder,
    component::{Component, HasSelf, Linker},
};
use wasmtime_wasi::{
    WasiCtx,
    cli::{StdinStream, StdoutStream},
    p2::bindings::Command,
};

use crate::{
    context::{HostCtx, HostFunctions},
    error::ExecutionError,
    outcome::{ExecutionDetails, ExecutionOutcome},
};

/// Size of the guard region placed before linear memory to
/// catch out-of-bounds accesses
const MEMORY_GUARD_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB

/// Maximum fuel consumption allowed for a single component execution
const MAX_FUEL_CONSUMPTION: u64 = u64::MAX;

pub struct ExecutionContext<I, O, E, H> {
    stdin: I,
    stdout: O,
    stderr: E,
    host_state: H,
    fuel_limit: u64,
    memory_limit: Option<usize>,
}

impl<I, O, E, H> ExecutionContext<I, O, E, H>
where
    I: StdinStream + 'static,
    O: StdoutStream + 'static,
    E: StdoutStream + 'static,
    H: HostFunctions,
{
    /// Creates a new execution context
    /// By default, there are no fuel or memory limits
    #[must_use]
    pub const fn new(stdin: I, stdout: O, stderr: E, host_state: H) -> Self {
        Self {
            stdin,
            stdout,
            stderr,
            host_state,
            fuel_limit: MAX_FUEL_CONSUMPTION,
            memory_limit: None,
        }
    }

    /// Sets the maximum fuel the component may consume per execution
    ///
    /// Fuel roughly corresponds to the number of WebAssembly instructions
    /// executed. Exceeding the limit raises [`ExecutionError::Trap`].
    #[must_use]
    pub const fn fuel_limit(mut self, fuel_limit: u64) -> Self {
        self.fuel_limit = fuel_limit;
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

    pub(crate) fn into_store(self, engine: &Engine) -> Store<HostCtx<H>> {
        let Self {
            stdin,
            stdout,
            stderr,
            host_state,
            fuel_limit,
            memory_limit,
        } = self;

        let wasi_ctx = WasiCtx::builder()
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr)
            .allow_udp(false)
            .allow_tcp(false)
            // TODO: other settings, such as:
            // * .secure_random()
            .build();

        let mut limits = StoreLimitsBuilder::new();
        if let Some(memory_limit) = memory_limit {
            limits = limits.memory_size(memory_limit);
        }

        let mut store = Store::new(engine, HostCtx::new(wasi_ctx, host_state, limits.build()));

        store.limiter(|ctx| ctx.limits_mut());
        store.set_fuel(fuel_limit).expect("fuel must be enabled");

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
    /// let stdout = MemoryOutputPipe::new(4 * 1024 * 1024); // 4 MB
    /// let stderr = MemoryOutputPipe::new(64 * 1024);       // 64 KB
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
        ctx: ExecutionContext<I, O, E, H>,
        component: &Component,
    ) -> Result<ExecutionOutcome>
    where
        I: StdinStream + 'static,
        O: StdoutStream + 'static,
        E: StdoutStream + 'static,
    {
        let fuel_limit = ctx.fuel_limit;
        let mut store = ctx.into_store(self.linker.engine());

        let command = Command::instantiate_async(&mut store, component, &self.linker)
            .await
            .context("instantiate")?;

        let run_result = command.wasi_cli_run().call_run(&mut store).await;

        Ok(ExecutionOutcome {
            error: match run_result {
                Ok(Ok(())) => None,
                Ok(Err(())) => Some(ExecutionError::Unknown(anyhow!("wasm component failed"))),
                Err(trap) => ExecutionError::from_trap(trap),
            },
            details: ExecutionDetails {
                fuel_consumed: fuel_limit
                    .saturating_sub(store.get_fuel().expect("fuel must be enabled")),
            },
        })
    }

    /// Convenience method to compile and execute a component in one step. See
    /// [`execute`](Self::execute) for details and examples.
    pub async fn run<I, O, E>(
        &self,
        ctx: ExecutionContext<I, O, E, H>,
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
