use crate::csv_reader::{async_read_csv, CsvReadError};
use std::env;
use std::io::Write;
use thiserror::Error;
use tokio::task;

mod csv_reader;

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    CsvRead(#[from] CsvReadError),
    #[error("Exactly one argument must be provided to represent the CSV path.")]
    ArgsLengthError,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Read CSV file path from command line arguments, assume single CSV being supplied
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err(AppError::ArgsLengthError);
    }

    let csv_path = args[1].clone();

    let reader_handle = task::spawn(async_read_csv(csv_path));

    // Wait for the reader task to finish
    let records = reader_handle.await.unwrap()?;

    // Output the records
    for record in records {
        println!("{:?}", record);
    }

    Ok(())
}
