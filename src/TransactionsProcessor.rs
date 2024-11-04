use crate::domain::{TransactionRecord, TransactionType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

type ClientId = u16;
type TransactionId = u32;
#[derive(Debug, Clone)]
pub struct Account {
    available: f32,
    held: f32,
    total: f32,
    locked: bool,
}

type SharedMap<K, V> = Arc<RwLock<HashMap<K, V>>>;

pub struct TransactionProcessor {
    client_accounts: SharedMap<ClientId, Account>,
    transactions_history: SharedMap<TransactionId, TransactionRecord>,
    max_workers: usize,
}

impl TransactionProcessor {
    /// Creates a new TransactionProcessor instance
    pub fn new(max_workers: usize) -> Self {
        Self {
            client_accounts: Arc::new(RwLock::new(HashMap::new())),
            transactions_history: Arc::new(RwLock::new(HashMap::new())),
            max_workers,
        }
    }

    /// Inserts a client account or update it if exists based on transaction record
    pub async fn insert_or_update_client_account(&self, transaction: TransactionRecord) {
        unimplemented!()
    }

    /// Fetches a transaction from the transactions_history map
    pub async fn get_transaction(
        &self,
        transaction_id: TransactionId,
    ) -> Option<TransactionRecord> {
        unimplemented!()
    }

    /// Processes a batch of transactions by client, managing concurrency with a maximum worker limit.
    pub async fn process_records_batch(&self, records: Vec<TransactionRecord>) {
        unimplemented!()
    }

    /// Core processing logic for client records, updating account balances and transaction history.
    /// Despotic: add to available balance
    /// Withdraw: subtract from available balance and total balance. If available balance results in
    ///     negative value then ignore the record
    /// Dispute: take from available funds the amount and add it to held funds.
    /// Resolve: Lookup the transaction and deduct amount from held funds to available funds. If
    ///     transaction does not exist, ignore the record.
    /// ChargeBack: Lookup the transaction and deduct amount from held and total balance
    pub(crate) async fn process_client_records(
        client_id: ClientId,
        records: Vec<TransactionRecord>,
        client_accounts: SharedMap<ClientId, Account>,
        transactions_history: SharedMap<TransactionId, TransactionRecord>,
    ) {
        for record in records {
            // Insert transaction record into history
            let mut history = transactions_history
                .write()
                .await;
            history.insert(record.transaction_id, record);
            drop(history); // Release lock on transactions_history early

            // Process the record and update client account
            let mut accounts = client_accounts.write().await;
            if let Some(account) = accounts.get_mut(&client_id) {
                match record.transaction_type {
                    TransactionType::Deposit => account.available += record.amount,
                    TransactionType::Withdrawal => account.available -= record.amount,
                    TransactionType::Dispute => {
                        // Custom logic for resolving disputes
                    }
                    TransactionType::Resolve => {
                        // Custom logic for chargebacks
                    }
                    TransactionType::Chargeback => {
                        // Custom logic for chargebacks
                    }
                }
                account.total = account.available + account.held;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_process_client_records() {
        // Arrange
        let client_id = 1;
        let transaction_id_1 = 1;
        let transaction_id_2 = 2;
        let transaction_id_3 = 3;
        let transaction_id_4 = 4;

        let record1 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id,
            transaction_id: transaction_id_1,
            amount: 1.9234,
        };
        let record2 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id,
            transaction_id: transaction_id_2,
            amount: 23.352_577,
        };
        let record3 = TransactionRecord {
            transaction_type: TransactionType::Withdrawal,
            client_id,
            transaction_id: transaction_id_3,
            amount: 50.234_35,
        };
        let record4 = TransactionRecord {
            transaction_type: TransactionType::Withdrawal,
            client_id,
            transaction_id: transaction_id_4,
            amount: 20.435434,
        };
        let records = vec![record1, record2, record3, record4];

        // Initialize shared maps for client_accounts and transactions_history
        let client_accounts: SharedMap<ClientId, Account> = Arc::new(RwLock::new(HashMap::new()));
        let transactions_history: SharedMap<TransactionId, TransactionRecord> =
            Arc::new(RwLock::new(HashMap::new()));

        // Insert initial account for the client
        {
            let mut accounts = client_accounts.write().await;
            accounts.insert(
                client_id,
                Account {
                    available: 0.0,
                    held: 0.0,
                    total: 0.0,
                    locked: false,
                },
            );
        }

        // Run the function
        TransactionProcessor::process_client_records(
            client_id,
            records,
            Arc::clone(&client_accounts),
            Arc::clone(&transactions_history),
        )
        .await;

        // Verify the updated account state
        {
            let accounts = client_accounts.read().await;
            let account = accounts
                .get(&client_id)
                .expect("Account not found");

            assert_eq!(account.available, 1.0);
            assert_eq!(account.total, 1.0);
            assert_eq!(account.available, 1.0);
            assert!(!account.locked);
        }

        // Verify the transaction history
        {
            let history = transactions_history
                .read()
                .await;
            assert_eq!(history.get(&transaction_id_1), Some(&record1));
            assert_eq!(history.get(&transaction_id_2), Some(&record2));
            assert_eq!(history.get(&transaction_id_3), Some(&record1));
            assert_eq!(history.get(&transaction_id_4), Some(&record2));
        }
    }
}
