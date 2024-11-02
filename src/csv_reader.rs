use csv::{ReaderBuilder, Trim};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use thiserror::Error;

#[derive(serde::Deserialize, Debug)]
pub struct TransactionRecord {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    #[serde(rename = "client")]
    client_id: u16,
    #[serde(rename = "tx")]
    transaction_id: u32,
    #[serde(rename = "amount")]
    amount: f32,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

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

pub async fn async_read_csv(path: String) -> Result<Vec<TransactionRecord>, CsvReadError> {
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

    Ok(records)
}
