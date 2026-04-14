use crate::Host;

pub const HOST_SEED_ENV: &str = "HOST_SEED";
const DEFAULT_HOST_SEED: &str = "default_seed";

pub fn init_host() -> Host {
    let seed = std::env::var(HOST_SEED_ENV);

    defuse_outlayer_worker_host::WorkerHost::from_seed(
        seed.as_deref().unwrap_or(DEFAULT_HOST_SEED).as_bytes(),
    )
}
