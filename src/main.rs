use crate::csv_reader::{read_csv, CsvReadError};
use crate::transactions_manager::TransactionsManager;
use crate::transactions_queue::TransactionsQueue;
use std::env;
use std::sync::Arc;
use thiserror::Error;
use tokio::task;

mod TransactionsProcessor;
mod csv_reader;
mod domain;
mod transactions_manager;
mod transactions_queue;

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    CsvRead(#[from] CsvReadError),
    #[error("Exactly one argument must be provided to represent the CSV path.")]
    ArgsLengthError,
}

///
/// 1. Init transaction_manager
///     a. starts an async loop within `listen_for_incoming_client_transactions`
///     b. returns an instance of `TransactionQueue` to be passed to `read_csv` task (see below)
///
/// 2. Async Start `read_csv` passing it `csv_path` from command args and `TransactionQueue`.
///
/// 3. At the end of the main, make sure `read_csv` finishes and any task spawned by
/// `transactions_manager` is finished before exiting.
///
#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Read CSV file path from command line arguments, assume single CSV being supplied
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err(AppError::ArgsLengthError);
    }

    // 1. Start TransactionsManager
    let TransactionsManager {
        queue,
        sender,
        receiver,
    } = TransactionsManager::new();

    let csv_path = args[1].clone();

    let reader_handle =
        task::spawn_blocking(move || read_csv(csv_path, Arc::clone(&queue), sender));

    // 3.
    // Wait for the reader task to finish
    let records = reader_handle
        .await
        .unwrap()
        .await?;

    // Output the records
    // for record in records {
    //     println!("{:?}", record);
    // }

    Ok(())
}
