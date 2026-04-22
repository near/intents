use wasmtime::Trap;

/// Errors that can occur during execution of a WASI component
#[derive(thiserror::Error, Debug)]
pub enum ExecutionError {
    /// The component called `process::exit` with a non-zero exit code
    #[error("component exited with code {code}")]
    NonZeroExit { code: i32 },

    /// The component raised a WebAssembly trap
    #[error("wasm trap: {code}")]
    Trap { code: Trap },

    /// An error that does not map to a known trap code
    #[error("component trapped: {source}")]
    Unknown {
        #[source]
        source: anyhow::Error,
    },
}
