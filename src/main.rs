use crate::csv_reader::CsvReadError;
use std::env;
use std::io::Write;
use thiserror::Error;

mod csv_reader;

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    CsvRead(#[from] CsvReadError),
    #[error("Exactly one argument must be provided to represent the CSV path.")]
    ArgsLengthError,
}
fn main() -> Result<(), AppError> {
    // Read CSV file path from command line arguments, assume single CSV being supplied
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err(AppError::ArgsLengthError);
    }

    let csv_path = &args[1];

    let records = csv_reader::read_csv(csv_path)?;

    // Output the records
    for record in records {
        println!("{:?}", record);
    }
    Ok(())
}
