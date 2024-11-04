use crate::domain::TransactionRecord;
use crate::transactions_queue::TransactionsQueue;
use csv::{ReaderBuilder, Trim};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;

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

pub async fn read_csv(
    path: String,
    transactions_queue: Arc<TransactionsQueue>,
    sender: mpsc::Sender<u16>,
) -> Result<Vec<TransactionRecord>, CsvReadError> {
    let file = open_csv(path)?;
    let reader = BufReader::new(file);
    let mut records: Vec<TransactionRecord> = Vec::new();

    // by default no header is read, so follow requirements
    let mut csv_reader = ReaderBuilder::new()
        .trim(Trim::All)
        .from_reader(reader);

    for result in csv_reader.deserialize::<TransactionRecord>() {
        match result {
            Ok(record) => {
                // transactions_queue
                //     .enqueue(record.client_id, record, sender.clone())
                //     .await;
                let sender = sender.clone();
                tokio::spawn(async move {
                    println!("processing record {:?}", record);
                });
                records.push(record)
            }
            Err(err) => return Err(CsvReadError::IoReadError(err.to_string())),
        }
    }

    println!("returning from read_csv");
    Ok(records)
}
