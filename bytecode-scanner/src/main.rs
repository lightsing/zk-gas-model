use alloy::{network::Ethereum, providers::Provider};
use clap::Parser;
use eyre::Result;
use sqlx::SqlitePool;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    /// Path to the bytecode storage
    #[clap(long, env = "BYTECODE_STORE_PATH", default_value = "bytecodes.sqlite")]
    store: PathBuf,
    /// URL of the RPC endpoint
    #[clap(long, env = "ETHEREUM_RPC_URL")]
    rpc: url::Url,

    #[clap(long, env = "CHECKPOINTS_DB_PATH", default_value = "checkpoints")]
    checkpoints: PathBuf,

    /// Requests per second to throttle
    #[arg(long, default_value = "5", env = "ETHEREUM_RPC_REQUESTS_PER_SECOND")]
    requests_per_second: u32,

    /// Maximum number of retries for rate limiting error
    #[arg(
        long,
        default_value = "10",
        env = "ETHEREUM_RPC_MAX_RATE_LIMIT_RETRIES"
    )]
    max_rate_limit_retries: u32,
    /// Initial backoff time in seconds for retrying requests
    #[arg(long, default_value = "1", env = "ETHEREUM_RPC_INITIAL_BACKOFF")]
    initial_backoff: u64,
    /// Compute units per second for the RPC provider
    #[arg(
        long,
        default_value = "170",
        env = "ETHEREUM_RPC_COMPUTE_UNITS_PER_SECOND"
    )]
    compute_units_per_second: u64,
}

struct AppData<P: Provider<Ethereum>> {
    db: SqlitePool,
    provider: P,
    checkpoints: sled::Tree,
}

mod init;
mod scanner;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;

    let app_data = init::init(Args::parse()).await?;
    scanner::start(app_data).await
}
