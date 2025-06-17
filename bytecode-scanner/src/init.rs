use crate::{AppData, Args};
use alloy::{
    providers::{Provider, ProviderBuilder},
    rpc::client::ClientBuilder,
};
use alloy_transport::layers::{RetryBackoffLayer, ThrottleLayer};
use eyre::{Result, WrapErr};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{path::Path, sync::Arc};

pub async fn init(args: Args) -> Result<Arc<AppData<impl Provider + 'static>>> {
    let db = open_bytecode_store(&args.store).await?;
    let provider = open_provider(&args);

    let chain_id = provider
        .get_chain_id()
        .await
        .wrap_err("failed to get chain id")?;
    let checkpoints = sled::open(args.checkpoints.as_path())?;
    let checkpoints = checkpoints
        .open_tree(format!("chain_id_{}", chain_id))
        .wrap_err("failed to open checkpoints tree")?;

    Ok(Arc::new(AppData {
        db,
        provider,
        checkpoints,
    }))
}

async fn open_bytecode_store(store_path: &Path) -> Result<SqlitePool> {
    let db = SqlitePoolOptions::new()
        .connect_with(
            SqliteConnectOptions::new()
                .filename(store_path)
                .create_if_missing(true),
        )
        .await
        .wrap_err("failed to open to SQLite database")?;
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .wrap_err("failed to run migrations")?;
    Ok(db)
}

fn open_provider(args: &Args) -> impl Provider + 'static {
    let client = ClientBuilder::default()
        .layer(RetryBackoffLayer::new(
            args.max_rate_limit_retries,
            args.initial_backoff,
            args.compute_units_per_second,
        ))
        .layer(ThrottleLayer::new(args.requests_per_second))
        .http(args.rpc.clone());
    ProviderBuilder::new().connect_client(client)
}
