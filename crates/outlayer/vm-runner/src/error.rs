use wasmtime::Trap;

/// Errors that can occur during execution of a WASI component
#[derive(thiserror::Error, Debug)]
pub enum VmError {
    /// An error that occurred during initialization of the runtime
    #[error("initialization error: {0}")]
    Init(#[source] anyhow::Error),

    /// An error that occurred during compilation of the component
    #[error("compilation error: {0}")]
    Compile(#[source] anyhow::Error),

    /// The component called `process::exit` with a non-zero exit code
    #[error("component exited with code {0}")]
    NonZeroExit(i32),

    /// The component raised a WebAssembly trap
    #[error("wasm trap: {0}")]
    Trap(Trap),

    /// An error that does not map to a known trap code
    #[error("component trapped: {0}")]
    Unknown(#[source] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, VmError>;
