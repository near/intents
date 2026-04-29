pub mod crypto;

#[cfg(not(target_family = "wasm"))] // TODO: or feature?
mod mock;
