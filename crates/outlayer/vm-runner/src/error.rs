pub use wasmtime::Trap;

/// Errors that can occur during execution of a WASI component
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum ExecutionError {
    /// The component called `process::exit` with exit code
    #[error("component exited with code: {0}")]
    NonZeroExit(i32),

    /// The component raised a WebAssembly trap
    #[error(transparent)]
    Trap(#[from] Trap),

    /// An error that does not map to a known trap code
    #[error(transparent)]
    Custom(anyhow::Error),
}

impl ExecutionError {
    pub fn from_trap(trap: anyhow::Error) -> Option<Self> {
        if let Some(exit) = trap.downcast_ref::<wasmtime_wasi::I32Exit>() {
            return Self::exit_code(exit.0);
        }

        Some(
            trap.downcast::<Trap>()
                .map_or_else(Self::Custom, Self::Trap),
        )
    }

    pub const fn exit_code(exit: i32) -> Option<Self> {
        if exit == 0 {
            return None;
        }
        Some(Self::NonZeroExit(exit))
    }
}
