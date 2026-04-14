use crate::Host;

pub const HOST_SEED_ENV: &str = "HOST_SEED";
const DEFAULT_HOST_SEED: &[u8] = b"default_seed";

pub fn init_host() -> Host {
    let seed = std::env::var(HOST_SEED_ENV).map_or(DEFAULT_HOST_SEED.to_vec(), |v| v.into_bytes());

    defuse_outlayer_worker_host::WorkerHost::from_seed(&seed)
}
