use wasmtime::Trap;

/// Errors that can occur during execution of a WASI component
#[derive(thiserror::Error, Debug)]
pub enum ExecutionError {
    /// The component called `process::exit` with exit code
    #[error("component exited with code {0}")]
    NonZeroExit(i32),

    /// The component raised a WebAssembly trap
    #[error("wasm trap: {0}")]
    Trap(Trap),

    #[error("component execution failed")]
    Failed,

    /// An error that does not map to a known trap code
    #[error("component trapped: {0}")]
    Unknown(#[source] anyhow::Error),
}
