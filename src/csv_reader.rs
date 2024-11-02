use csv::{ReaderBuilder, StringRecord, Trim};
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::Path;
use thiserror::Error;

const MAX_ROW_READS: usize = 3;

#[derive(Error, Debug)]
pub enum CsvReadError {
    #[error("Path to CSV is invalid!")]
    PathDoesNotExist,

    #[error("Failed to read CSV, print record that was a failure: {0}")]
    IoReadError(String),
}

pub fn open_csv(path: &str) -> Result<File, CsvReadError> {
    if !Path::new(path).exists() {
        return Err(CsvReadError::PathDoesNotExist);
    }

    File::open(path).map_err(|_| CsvReadError::IoReadError(path.to_string()))
}

pub fn read_csv_records<R: BufRead>(reader: R) -> Result<Vec<StringRecord>, CsvReadError> {
    let mut records: Vec<StringRecord> = Vec::new();

    // by default no header is read, so follow requirements
    let mut csv_reader = dbg!(ReaderBuilder::new().trim(Trim::All)).from_reader(reader);

    for result in csv_reader.records() {
        match result {
            Ok(record) => records.push(record),
            Err(err) => return Err(CsvReadError::IoReadError(err.to_string())),
        }
    }

    Ok(records)
}

// In your main read_csv function
pub fn read_csv(path: &str) -> Result<Vec<StringRecord>, CsvReadError> {
    let file = open_csv(path)?;
    let records = read_csv_records(io::BufReader::new(file))?;
    Ok(records)
}
