use wasmtime::Trap;

/// Errors that can occur during execution of
/// a component in the VM runtime
#[derive(thiserror::Error, Debug)]
pub enum ExecutionError {
    #[error("component exited with code {code}\n{stderr}")]
    NonZeroExit { code: i32, stderr: String },

    #[error("wasm trap: {code}\n{stderr}")]
    Trap { code: Trap, stderr: String },

    #[error("component trapped: {source}\n{stderr}")]
    Unknown {
        #[source]
        source: anyhow::Error,
        stderr: String,
    },
}
