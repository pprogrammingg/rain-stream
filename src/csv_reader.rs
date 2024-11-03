use crate::domain::TransactionRecord;
use crate::transactions_queue::TransactionsQueue;
use csv::{ReaderBuilder, Trim};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

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
) -> Result<Vec<TransactionRecord>, CsvReadError> {
    let file = open_csv(path)?;
    let reader = BufReader::new(file);
    let mut records: Vec<TransactionRecord> = Vec::new();

    // by default no header is read, so follow requirements
    let mut csv_reader = ReaderBuilder::new()
        .trim(Trim::All)
        .from_reader(reader);

    for result in csv_reader.deserialize() {
        match result {
            Ok(record) => records.push(record),
            Err(err) => return Err(CsvReadError::IoReadError(err.to_string())),
        }
    }

    println!("Returning from read_csv");
    Ok(records)
}
