use defuse_outlayer_host::State as HostState;

pub struct WasiContext<I, O, E> {
    pub stdin: I,
    pub stdout: O,
    pub stderr: E,
}

pub struct Context<I, O, E> {
    pub wasi: WasiContext<I, O, E>,
    pub host_state: HostState,
    /// Maximum fuel the component may consume per execution
    ///
    /// Fuel roughly corresponds to the number of WebAssembly instructions
    /// executed. Exceeding the limit raises [`ExecutionError::Trap`].
    pub fuel: u64,
}
