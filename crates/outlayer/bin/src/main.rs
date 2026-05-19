use std::{sync::Arc, time::Duration};

use anyhow::{Context as _, Result};
use bytes::Bytes;
use figment::{
    Figment,
    providers::{Env, Serialized},
};
use serde::{Deserialize, Serialize};
use tower::service_fn;

use defuse_outlayer_executor::{Executor, Config as ExecutorConfig, ExecutorLimits};
use defuse_outlayer_service::{
    CacheBuilder, CacheConfig, Code, HttpResolver, NearResolver, Outlayer, Resolver,
    ResolverConfig, UrlResolver,
};
use defuse_outlayer_vm_runner::{VmRuntime, host::InMemorySigner};
use near_kit::Near;

#[derive(Deserialize, Serialize)]
struct AppConfig {
    executor: ExecutorConfig,
    cache: CacheConfig,
    resolver: ResolverConfig,
    default_fuel: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            executor: ExecutorConfig::default(),
            cache: CacheConfig::default(),
            resolver: ResolverConfig::default(),
            default_fuel: u64::MAX,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("RUST_LOG"))
        .init();

    let config: AppConfig = Figment::new()
        .merge(Serialized::defaults(AppConfig::default()))
        .merge(Env::prefixed("OUTLAYER__").split("__"))
        .extract()
        .context("config")?;

    let seed = match config.executor.signer_seed {
        Some(ref s) => hex::decode(s).context("OUTLAYER__EXECUTOR__SIGNER_SEED: invalid hex")?,
        None => b"test".to_vec(),
    };

    let executor = {
        let signer = InMemorySigner::from_seed(&seed);
        let limits = ExecutorLimits {
            stdin: config.executor.stdin_limit,
            stdout: config.executor.stdout_limit,
            stderr: config.executor.stderr_limit,
        };
        let runtime =
            Arc::new(VmRuntime::new(config.executor.memory_limit).context("VmRuntime")?);
        Executor::new(runtime, signer, limits)
    };

    let resolver = {
        let near =
            Near::custom(config.resolver.near_rpc_url, config.resolver.near_chain_id).build();
        Resolver::new(
            NearResolver::new(near),
            UrlResolver::new(HttpResolver::new(config.resolver.http_max_len)),
        )
    };

    let mut cache = CacheBuilder::default().max_capacity(config.cache.max_capacity);
    if let Some(tti) = config.cache.tti_secs {
        cache = cache.time_to_idle(Duration::from_secs(tti));
    }

    let outlayer = Outlayer::builder().with_cache(cache).build(resolver, executor);

    let default_fuel = config.default_fuel;
    let _svc = service_fn(move |(app, input): (Code<'static>, Bytes)| {
        let outlayer = outlayer.clone();
        async move { outlayer.execute(app, input, default_fuel).await }
    });
    Ok(())
}
