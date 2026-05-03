use defuse_outlayer_vm_runner::wasmtime;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // TODO: tell the limit in err text
    #[error("input is too long")]
    InputTooLong,

    #[error("compile: {0}")]
    Compile(wasmtime::Error),

    #[error("execute: {0}")]
    Execute(wasmtime::Error),
}
