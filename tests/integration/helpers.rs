use std::{
    error::Error,
    fs::File,
    path::{
        Path,
        PathBuf,
    },
};

use csv::Writer;

#[derive(Debug)]
pub struct Transaction {
    pub tx_type: String,
    pub client_id: u32,
    pub tx_id: u32,
    pub amount: f64,
}

pub const CSV_FOLDER_PATH: &str = "test_csv";

pub fn generate_csv_input(
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

pub fn generate_transactions(n: u32) -> Vec<Transaction> {
    let mut transactions = Vec::new();
    let mut transaction_id = 1;

    // Create 5 deposits for clients 1 and 2
    for client_id in 1..=2 {
        for _ in 0..n / 8 {
            transactions.push(Transaction {
                tx_type: "deposit".to_string(),
                client_id,
                tx_id: transaction_id,
                amount: 100.0,
            });
            transaction_id += 1;
        }
    }

    // clients 1 and 2
    for client_id in 1..=2 {
        for _ in 0..n / 8 {
            transactions.push(Transaction {
                tx_type: "withdrawal".to_string(),
                client_id,
                tx_id: transaction_id,
                amount: 50.0,
            });
            transaction_id += 1;
        }
    }

    // client 3
    for _ in 0..n / 4 {
        transactions.push(Transaction {
            tx_type: "deposit".to_string(),
            client_id: 3,
            tx_id: transaction_id,
            amount: 50.0,
        });
        transaction_id += 1;
    }

    // Dispute a transaction for client 3
    let disputed_transaction = transaction_id - 2;
    transactions.push(Transaction {
        tx_type: "dispute".to_string(),
        client_id: 3,
        tx_id: disputed_transaction,
        amount: 0.0,
    });

    // Resolve the transaction for client 3
    transactions.push(Transaction {
        tx_type: "resolve".to_string(),
        client_id: 3,
        tx_id: disputed_transaction,
        amount: 0.0,
    });

    // client 4
    for _ in 0..n / 4 {
        transactions.push(Transaction {
            tx_type: "deposit".to_string(),
            client_id: 4,
            tx_id: transaction_id,
            amount: 200.0,
        });
        transaction_id += 1;
    }

    // Dispute a transaction for client 4
    let disputed_transaction = transaction_id - 2;
    transactions.push(Transaction {
        tx_type: "dispute".to_string(),
        client_id: 4,
        tx_id: disputed_transaction,
        amount: 0.0,
    });

    // Chargeback the disputed transaction
    transactions.push(Transaction {
        tx_type: "chargeback".to_string(),
        client_id: 4,
        tx_id: disputed_transaction,
        amount: 0.0,
    });

    // Further transactions for client 4
    transactions.push(Transaction {
        tx_type: "deposit".to_string(),
        client_id: 4,
        tx_id: transaction_id,
        amount: 10000.0,
    });

    transactions
}
