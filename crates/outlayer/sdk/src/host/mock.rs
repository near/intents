use defuse_outlayer_worker_host::WorkerHost;

thread_local! {
    pub(super) static HOST: std::cell::RefCell<WorkerHost>
        = std::cell::RefCell::new(WorkerHost::from_seed([0u8; 32]));
}

// const OUTLAYER_SEED_ENV_VAR: &str = "OUTLAYER_SEED";

// fn seed() -> [u8; 32] {
//     std::env::var_os(OUTLAYER_SEED_ENV_VAR)
//         .map(|s| hex::decode(s.as_encoded_bytes()).expect("hex"))
//         .map(|b| b.try_into().expect("invalid length"))
//         .unwrap_or_default()
// }

// fn from_env() -> WorkerHost {
//     WorkerHost::from_seed(seed())
// }
