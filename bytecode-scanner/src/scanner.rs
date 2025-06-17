use crate::AppData;
use alloy::{
    consensus::Transaction,
    eips::BlockNumberOrTag,
    primitives::{Address, B256},
    providers::Provider,
};
use eyre::{ContextCompat, Result, WrapErr};
use revm_bytecode::Bytecode;
use std::sync::{Arc, atomic::AtomicBool};

pub async fn start(app_data: Arc<AppData<impl Provider + 'static>>) -> Result<()> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::task::spawn({
        let shutdown = shutdown.clone();
        async move {
            tokio::signal::ctrl_c().await.unwrap();
            shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    });

    let saver_worker = tokio::task::spawn({
        let app_data = app_data.clone();
        saver(app_data, rx)
    });

    'outer: loop {
        let mut block_number = app_data
            .provider
            .get_block_number()
            .await
            .wrap_err("failed to get latest block")?;
        while block_number > 0 {
            if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                break 'outer;
            }
            if check_block_fetched(&app_data, block_number)? {
                block_number -= 1;
                continue;
            }

            let block = app_data
                .provider
                .get_block_by_number(BlockNumberOrTag::Number(block_number))
                .full()
                .await
                .wrap_err("failed to get block by number")?
                .wrap_err("fetched empty block")?;

            let txs = block
                .transactions
                .into_transactions()
                .into_iter()
                .filter_map(|tx| tx.to());

            for address in txs {
                if check_address_fetched(&app_data, address)? {
                    continue;
                }
                let code = app_data
                    .provider
                    .get_code_at(address)
                    .number(block_number)
                    .await
                    .wrap_err("failed to get code")?;
                let bytecode = Bytecode::new_raw(code);
                let hash = bytecode.hash_slow();
                mark_address_fetched(&app_data, address)?;
                tx.send((hash, bytecode))
                    .wrap_err("failed to send to saver")?;
            }
            mark_block_fetched(&app_data, block_number)?;
        }
    }

    saver_worker.await.wrap_err("failed to await saver")??;
    Ok(())
}

async fn saver(
    app_data: Arc<AppData<impl Provider>>,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<(B256, Bytecode)>,
) -> Result<()> {
    while let Some((hash, bytecode)) = rx.recv().await {
        if update_count_if_bytecode_exist(&app_data, hash).await? {
            continue;
        }
        store_bytecode(&app_data, hash, bytecode).await?;
    }

    Ok(())
}

#[inline]
fn mark_address_fetched(app_data: &AppData<impl Provider>, address: Address) -> Result<()> {
    app_data
        .checkpoints
        .insert(address, &[1u8])
        .wrap_err("failed to mark address as fetched")?;
    Ok(())
}

#[inline]
fn check_address_fetched(app_data: &AppData<impl Provider>, address: Address) -> Result<bool> {
    Ok(app_data
        .checkpoints
        .get(address)
        .wrap_err("failed to read checkpoints")?
        .is_some())
}

#[inline]
fn mark_block_fetched(app_data: &AppData<impl Provider>, block_number: u64) -> Result<()> {
    app_data
        .checkpoints
        .insert(block_number.to_le_bytes(), &[1u8])
        .wrap_err("failed to mark block as fetched")?;
    Ok(())
}

#[inline]
fn check_block_fetched(app_data: &AppData<impl Provider>, block_number: u64) -> Result<bool> {
    Ok(app_data
        .checkpoints
        .get(block_number.to_le_bytes())
        .wrap_err("failed to read checkpoints")?
        .is_some())
}

#[inline]
async fn update_count_if_bytecode_exist(
    app_data: &AppData<impl Provider>,
    hash: B256,
) -> Result<bool> {
    let hash = hash.as_slice();

    let rows_affected = sqlx::query!(
        "UPDATE bytecode SET call_counter = call_counter + 1 WHERE hash = ?",
        hash,
    )
    .execute(&app_data.db)
    .await
    .wrap_err("Failed to update bytecode call counter")?
    .rows_affected();

    Ok(rows_affected != 0)
}

#[inline]
async fn store_bytecode(
    app_data: &AppData<impl Provider>,
    hash: B256,
    bytecode: Bytecode,
) -> Result<()> {
    let hash = hash.as_slice();
    let bytecode = bytecode.original_byte_slice();
    sqlx::query!(
        "INSERT INTO bytecode (hash, bytecode) VALUES (?, ?)",
        hash,
        bytecode,
    )
    .execute(&app_data.db)
    .await
    .wrap_err("Failed to store bytecode")?;

    Ok(())
}
