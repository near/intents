use anyhow::Result;
use defuse_outlayer_host_functions::{HostFunctions, Imports};
use std::marker::PhantomData;
use tracing::instrument;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store, StoreLimitsBuilder, Trap};
use wasmtime_wasi::cli::{StdinStream, StdoutStream};
use wasmtime_wasi::{WasiCtx, p2::bindings::Command};

use crate::error::ExecutionError;
use crate::host::HostCtx;
use crate::outcome::ExecutionOutcome;

/// Size of the guard region placed before linear memory to
/// catch out-of-bounds accesses
const MEMORY_GUARD_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB

/// Default maximum physical memory for a single component
/// execution (100 MiB)
pub const DEFAULT_MEMORY_LIMIT: usize = 100 * 1024 * 1024;

/// Default fuel budget for a single component execution
/// (~1 billion wasm instructions)
pub const DEFAULT_FUEL_LIMIT: u64 = 1_000_000_000;

pub struct Context<I, O, E, H> {
    input: I,
    output: O,
    error: E,
    host_state: H,
}

impl<I, O, E, H> Context<I, O, E, H>
where
    I: StdinStream + 'static,
    O: StdoutStream + 'static,
    E: StdoutStream + 'static,
    H: HostFunctions,
{
    pub const fn new(input: I, output: O, error: E, host_state: H) -> Self {
        Self {
            input,
            output,
            error,
            host_state,
        }
    }
}

/// A builder for configuring and creating a `VmRuntime`
/// with a custom host environment
pub struct VmRuntimeBuilder<H: HostFunctions + 'static> {
    config: Config,
    fuel_limit: u64,
    memory_limit: usize,
    _host: PhantomData<H>,
}

impl<H: HostFunctions + 'static> VmRuntimeBuilder<H> {
    /// Creates a new builder with default configuration.
    ///
    /// Async support and fuel metering are always enabled and cannot be
    /// disabled. Use [`fuel_limit`](Self::fuel_limit) and
    /// [`memory_limit`](Self::memory_limit) to override the defaults
    pub fn new() -> Self {
        let mut config = Config::new();
        config.guard_before_linear_memory(true);
        config.memory_guard_size(MEMORY_GUARD_SIZE);
        // NOTE: this is enabling async in host-defined functions
        config.async_support(true);
        config.consume_fuel(true);

        Self {
            config,
            fuel_limit: DEFAULT_FUEL_LIMIT,
            memory_limit: DEFAULT_MEMORY_LIMIT,
            _host: PhantomData,
        }
    }

    /// Sets the maximum fuel the component may consume per execution
    ///
    /// Fuel roughly corresponds to the number of WebAssembly instructions
    /// executed. Exceeding the limit raises [`ExecutionError::Trap`].
    /// Defaults to [`DEFAULT_FUEL_LIMIT`] if not set
    #[must_use]
    pub const fn fuel_limit(mut self, fuel_limit: u64) -> Self {
        self.fuel_limit = fuel_limit;
        self
    }

    /// Sets the maximum physical memory the components linear memory may use
    ///
    /// Attempts to grow beyond this limit will trap. Defaults to
    /// [`DEFAULT_MEMORY_LIMIT`] if not set
    #[must_use]
    pub const fn memory_limit(mut self, memory_limit: usize) -> Self {
        self.memory_limit = memory_limit;
        self
    }

    /// Builds the `VmRuntime` with the specified configuration
    /// If fuel limit or memory limit are not set, default values will be used
    pub fn build(self) -> anyhow::Result<VmRuntime<H>> {
        // TODO: uncomment in corresponding pr
        // // NOTE: set initial chunk of virtual memory that a linear memory
        // // may grow into limited to allow multiple linear memories to be
        // // instantiated without exhausting host resources
        // self.config
        //     .memory_reservation(self.memory_limit.try_into().unwrap());

        let engine = Engine::new(&self.config)?;
        let linker = Self::create_linker(&engine)?;

        Ok(VmRuntime {
            fuel_limit: self.fuel_limit,
            memory_limit: self.memory_limit,
            linker,
        })
    }

    fn create_linker(engine: &Engine) -> anyhow::Result<Linker<HostCtx<H>>> {
        let mut linker = Linker::new(engine);

        // Add WASI imports to the linker
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

        // Add host function imports to the linker
        Imports::add_to_linker::<HostCtx<H>, HasSelf<H>>(&mut linker, |ctx: &mut HostCtx<H>| {
            ctx.host_state_mut()
        })?;

        Ok(linker)
    }
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
    fuel_limit: u64,
    memory_limit: usize,
    linker: Linker<HostCtx<H>>,
}

impl<H: HostFunctions + 'static> VmRuntime<H> {
    /// Creates a new `VmRuntime` with the default configuration
    pub fn new() -> anyhow::Result<Self> {
        Self::builder().build()
    }

    /// Creates a new builder for `VmRuntime`
    pub fn builder() -> VmRuntimeBuilder<H> {
        VmRuntimeBuilder::new()
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
    /// builder. Fuel exhaustion is reported as [`ExecutionError::Trap`]
    ///
    /// # Example
    ///
    /// Using [`HostState`] directly:
    ///
    /// ```rust,no_run
    /// use defuse_outlayer_state::HostState;
    /// use defuse_outlayer_vm_runner::{Context, VmRuntimeBuilder};
    /// use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let stdout = MemoryOutputPipe::new(4 * 1024 * 1024);
    /// let stderr = MemoryOutputPipe::new(64 * 1024);
    /// let ctx = Context::new(
    ///     MemoryInputPipe::new(b"input".to_vec()),
    ///     stdout.clone(),
    ///     stderr.clone(),
    ///     HostState::default(),
    /// );
    ///
    /// let runner = VmRuntimeBuilder::<HostState>::new().build()?;
    /// let wasm = std::fs::read("component.wasm")?;
    /// let component = runner.compile(&wasm)?;
    /// runner.execute(ctx, &component).await?;
    ///
    /// println!("{}", String::from_utf8_lossy(&stdout.contents()));
    /// # Ok(()) }
    /// ```
    ///
    /// You can also wrap [`HostState`] to share it across concurrent executions
    /// or inject additional context. Implement each host-function trait on your
    /// wrapper and delegate to the inner state:
    ///
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use defuse_outlayer_state::HostState;
    /// use defuse_outlayer_vm_runner::{Context, VmRuntimeBuilder};
    /// use tokio::sync::Mutex;
    /// use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
    ///
    /// struct Wrapper {
    ///     inner: Arc<Mutex<HostState>>,
    /// }
    ///
    /// impl defuse_outlayer_host_functions::crypto::ed25519::Host for Wrapper {
    ///     async fn derive_public_key(&mut self, path: String) -> Vec<u8> {
    ///         self.inner.lock().await.derive_public_key(path).await
    ///     }
    ///     async fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
    ///         self.inner.lock().await.sign(path, msg).await
    ///     }
    /// }
    ///
    /// impl defuse_outlayer_host_functions::crypto::secp256k1::Host for Wrapper {
    ///     async fn derive_public_key(&mut self, path: String) -> Vec<u8> {
    ///         self.inner.lock().await.derive_public_key(path).await
    ///     }
    ///     async fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
    ///         self.inner.lock().await.sign(path, msg).await
    ///     }
    /// }
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let shared = Arc::new(Mutex::new(HostState::default()));
    /// let stdout = MemoryOutputPipe::new(4 * 1024 * 1024);
    /// let stderr = MemoryOutputPipe::new(64 * 1024);
    /// let ctx = Context::new(
    ///     MemoryInputPipe::new(b"input".to_vec()),
    ///     stdout.clone(),
    ///     stderr.clone(),
    ///     Wrapper { inner: shared.clone() },
    /// );
    ///
    /// let runner = VmRuntimeBuilder::<Wrapper>::new().build()?;
    /// let wasm = std::fs::read("component.wasm")?;
    /// let component = runner.compile(&wasm)?;
    /// runner.execute(ctx, &component).await?;
    ///
    /// println!("{}", String::from_utf8_lossy(&stdout.contents()));
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
        let Context {
            input,
            output,
            error,
            host_state,
        } = ctx;

        let wasi_ctx = WasiCtx::builder()
            .stdin(input)
            .stdout(output)
            .stderr(error)
            .build();

        let limits = StoreLimitsBuilder::new()
            .memory_size(self.memory_limit)
            .build();
        let mut store = Store::new(
            self.linker.engine(),
            HostCtx::new(wasi_ctx, host_state, limits),
        );

        store.limiter(|ctx| ctx.limits_mut());
        store
            .set_fuel(self.fuel_limit)
            .expect("fuel must be enabled");

        let command = Command::instantiate_async(&mut store, component, &self.linker).await?;
        let run_result = command.wasi_cli_run().call_run(&mut store).await;

        let guest_error = match run_result {
            Ok(Ok(())) => None,
            Ok(Err(())) => Some(ExecutionError::Failed),
            Err(trap) => classify_error(trap),
        };

        let fuel_consumed = self
            .fuel_limit
            .saturating_sub(store.get_fuel().expect("fuel must be enabled"));

        Ok(ExecutionOutcome {
            fuel_consumed,
            guest_error,
        })
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
        let component = self.compile(binary)?;
        self.execute(ctx, &component).await
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

    let err = err
        .downcast_ref::<Trap>()
        .copied()
        .map_or_else(|| ExecutionError::Unknown(err), ExecutionError::Trap);

    tracing::debug!("Program trapped: {err}");

    Some(err)
}
