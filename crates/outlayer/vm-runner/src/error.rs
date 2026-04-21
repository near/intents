use wasmtime::Trap;

/// Errors that can occur during execution of a WASI component
#[derive(thiserror::Error, Debug)]
pub enum ExecutionError {
    /// The component called `process::exit` with a non-zero exit code
    #[error("component exited with code {code}\n{stderr}")]
    NonZeroExit { code: i32, stderr: String },

    /// The component raised a WebAssembly trap
    #[error("wasm trap: {code}\n{stderr}")]
    Trap { code: Trap, stderr: String },

    /// An error that does not map to a known trap code
    #[error("component trapped: {source}\n{stderr}")]
    Unknown {
        #[source]
        source: anyhow::Error,
        stderr: String,
    },
}
