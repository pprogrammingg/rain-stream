use crate::domain::TransactionRecord;
use crate::TransactionsProcessor::{
    Account, ClientId, SharedMap, TransactionId, TransactionProcessor,
};
use csv::{ReaderBuilder, Trim};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio::task;
use tokio::task::JoinHandle;

const CSV_RECORDS_CHUNK_SIZE: usize = 100;

#[derive(Error, Debug)]
pub enum CsvReadError {
    #[error("Path to CSV is invalid!")]
    PathDoesNotExist,

    #[error("Failed to read CSV, print record that was a failure: {0}")]
    IoReadError(String),
}

pub fn open_csv(path: String) -> Result<File, CsvReadError> {
    if !Path::new(&path).exists() {
        return Err(CsvReadError::PathDoesNotExist);
    }

    File::open(path.clone()).map_err(|_| CsvReadError::IoReadError(path))
}

///
/// Read CSV records in batches, for each chunk spawn a task to process the batch.
///
pub async fn read_csv(
    path: String,
    client_accounts: SharedMap<ClientId, Account>,
    transactions_history: SharedMap<TransactionId, TransactionRecord>,
    client_worker_map: SharedMap<ClientId, JoinHandle<()>>,
) -> Result<(), CsvReadError> {
    println!("Begin read_csv...");
    println!("+++++++++++++++");

    let file = open_csv(path)?;
    let reader = BufReader::new(file);
    let mut csv_reader = ReaderBuilder::new()
        .trim(Trim::All)
        .from_reader(reader);

    let mut records_batch = Vec::with_capacity(CSV_RECORDS_CHUNK_SIZE);

    for result in csv_reader.deserialize::<TransactionRecord>() {
        match result {
            Ok(record) => {
                records_batch.push(record);
                if records_batch.len() >= CSV_RECORDS_CHUNK_SIZE {
                    process_batch(
                        &records_batch,
                        &client_accounts,
                        &transactions_history,
                        &client_worker_map,
                    )
                    .await;
                    records_batch.clear();
                }
            }
            Err(err) => return Err(CsvReadError::IoReadError(err.to_string())),
        }
    }

    // Process remaining records if any
    if !records_batch.is_empty() {
        process_batch(
            &records_batch,
            &client_accounts,
            &transactions_history,
            &client_worker_map,
        )
        .await;
    }

    println!("Exiting read_csv...");
    println!("+++++++++++++++");

    Ok(())
}

// helper function to process batch
async fn process_batch(
    records_batch: &[TransactionRecord],
    client_accounts: &SharedMap<ClientId, Account>,
    transactions_history: &SharedMap<TransactionId, TransactionRecord>,
    client_worker_map: &SharedMap<ClientId, JoinHandle<()>>,
) {
    let batch_clone = records_batch.to_vec(); // Clone for safety in async context
    let client_accounts = Arc::clone(client_accounts);
    let transactions_history = Arc::clone(transactions_history);
    let client_worker_map = Arc::clone(client_worker_map);

    task::spawn_blocking(move || {
        // This will run on a separate blocking thread pool, so it won't block async tasks
        TransactionProcessor::process_records_batch(
            batch_clone,
            client_accounts,
            transactions_history,
            client_worker_map,
        )
    })
    .await
    .unwrap()
    .await; // Awaiting the blocking task
}
