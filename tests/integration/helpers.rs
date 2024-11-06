use csv::Writer;
use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Transaction {
    pub tx_type: String,
    pub client_id: u32,
    pub tx_id: u32,
    pub amount: f64,
}

pub const CSV_FOLDER_PATH: &str = "test_csv";

pub fn generate_sample_csv(
    file_name: &str,
    transactions: Vec<Transaction>,
) -> Result<PathBuf, Box<dyn Error>> {
    // Create "test_csv" folder if it doesn't exist
    let folder_path = Path::new(CSV_FOLDER_PATH);
    if !folder_path.exists() {
        std::fs::create_dir_all(folder_path)?;
    }

    // Create the CSV file path
    let file_path = folder_path.join(file_name);

    // Create the file
    let file = File::create(file_path.clone())?;
    let mut writer = Writer::from_writer(file);

    // Write the CSV header
    writer.write_record(["type", "client", "tx", "amount"])?;

    // Write the transactions to the CSV
    for transaction in transactions {
        writer.write_record([
            &transaction.tx_type,
            &transaction
                .client_id
                .to_string(),
            &transaction.tx_id.to_string(),
            &transaction.amount.to_string(),
        ])?;
    }

    // Flush and finalize the file
    writer.flush()?;

    Ok(file_path)
}

pub fn generate_sample_transactions() -> Vec<Transaction> {
    let mut transactions = Vec::new();

    // Client 1
    transactions.push(Transaction {
        tx_type: "deposit".to_string(),
        client_id: 1,
        tx_id: 1,
        amount: 1.0,
    });
    transactions.push(Transaction {
        tx_type: "deposit".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: 2.0,
    });
    transactions.push(Transaction {
        tx_type: "withdrawal".to_string(),
        client_id: 1,
        tx_id: 3,
        amount: 1.5,
    });

    // Client 2
    transactions.push(Transaction {
        tx_type: "deposit".to_string(),
        client_id: 2,
        tx_id: 4,
        amount: 2.0,
    });
    transactions.push(Transaction {
        tx_type: "deposit".to_string(),
        client_id: 2,
        tx_id: 5,
        amount: 3.0,
    });
    transactions.push(Transaction {
        tx_type: "withdrawal".to_string(),
        client_id: 2,
        tx_id: 6,
        amount: 1.0,
    });
    transactions.push(Transaction {
        tx_type: "withdrawal".to_string(),
        client_id: 2,
        tx_id: 7,
        amount: 1.0,
    });
    transactions.push(Transaction {
        tx_type: "withdrawal".to_string(),
        client_id: 2,
        tx_id: 8,
        amount: 1.0,
    });

    // Client 3
    transactions.push(Transaction {
        tx_type: "deposit".to_string(),
        client_id: 3,
        tx_id: 9,
        amount: 5.0,
    });
    transactions.push(Transaction {
        tx_type: "withdrawal".to_string(),
        client_id: 3,
        tx_id: 10,
        amount: 2.5,
    });

    transactions
}
