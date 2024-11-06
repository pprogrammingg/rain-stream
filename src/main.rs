use crate::csv_reader::{read_csv, CsvReadError};
use crate::domain::TransactionRecord;
use crate::transactions_queue::TransactionsQueue;
use crate::TransactionsProcessor::{Account, ClientId, SharedMap, TransactionId};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::task;
use tokio::task::JoinHandle;

mod TransactionsProcessor;
mod csv_reader;
mod domain;
mod transactions_manager;
mod transactions_queue;

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    CsvReadError(#[from] CsvReadError),
    #[error("Exactly one argument must be provided to represent the CSV path.")]
    ArgsLengthError,
}

///
/// 1. Instantiate TransactionProcessor
/// 2. Clone and pass in SharedMaps for client_accounts and transactions_history to read_csv
/// Note: read_csv should spawn as a separate task
#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Read CSV file path from command line arguments, assume a single CSV being supplied
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err(AppError::ArgsLengthError);
    }

    let csv_path = args[1].clone();
    println!("Processing CSV file at path: {}", csv_path);

    // Create shared maps to track status
    let client_accounts: SharedMap<ClientId, Account> = Arc::new(RwLock::new(HashMap::new()));
    let transactions_history: SharedMap<TransactionId, TransactionRecord> =
        Arc::new(RwLock::new(HashMap::new()));
    let client_worker_map: SharedMap<ClientId, JoinHandle<()>> =
        Arc::new(RwLock::new(HashMap::new()));

    // Spawn CSV reading and processing task
    let reader_handle = task::spawn({
        let client_accounts = Arc::clone(&client_accounts);
        let transactions_history = Arc::clone(&transactions_history);
        let client_worker_map = Arc::clone(&client_worker_map);
        async move {
            read_csv(
                csv_path,
                client_accounts,
                transactions_history,
                client_worker_map,
            )
            .await
        }
    });

    // Await the reader task's completion
    reader_handle.await.unwrap()?;

    // Print the final state of `client_accounts`
    println!("----------------------------");
    println!("Done, client_accounts is");
    print_client_accounts_2(&client_accounts).await;
    println!("____________________________");

    Ok(())
}

/// Prints `client_accounts` to stdout.
async fn print_client_accounts(client_accounts: &SharedMap<ClientId, Account>) {
    let client_accounts = client_accounts.read().await;

    println!("Final state of client_accounts:");
    for (client_id, account) in client_accounts.iter() {
        println!("Client ID: {:?}, Account: {:?}", client_id, account);
    }
}

async fn print_client_accounts_2(client_accounts: &SharedMap<ClientId, Account>) {
    let accounts = client_accounts.read().await;
    println!("{:?}", *accounts); // Replace with your account printing logic
}
