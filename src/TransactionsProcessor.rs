use crate::domain::{TransactionRecord, TransactionType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

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
}

const MAX_WORKERS: usize = 4;

impl TransactionProcessor {
    /// Creates a new TransactionProcessor instance
    pub fn new() -> Self {
        Self {
            client_accounts: Arc::new(RwLock::new(HashMap::new())),
            transactions_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Inserts a client account or update it if exists based on transaction record
    pub async fn insert_or_update_client_account(&self, transaction: TransactionRecord) {
        unimplemented!()
    }

    /// Fetches a transaction from the transactions_history map
    // pub async fn get_transaction(
    //     &self,
    //     transaction_id: TransactionId,
    // ) -> Option<TransactionRecord> {
    //     let history = self
    //         .transactions_history
    //         .read()
    //         .await;
    //
    //     history
    //         .get(&transaction_id)
    //         .cloned()
    // }

    pub async fn get_transaction(
        transactions_history: SharedMap<TransactionId, TransactionRecord>,
        transaction_id: TransactionId,
    ) -> Option<TransactionRecord> {
        let history = transactions_history
            .read()
            .await;

        history
            .get(&transaction_id)
            .cloned()
    }

    /// Insert a record in transaction_history
    // async fn insert_transaction_in_history(&self, transaction: TransactionRecord) {
    //     let mut history = self
    //         .transactions_history
    //         .write()
    //         .await;
    //     println!("Transactions history before insert: {:?}", history);
    //     history.insert(transaction.transaction_id, transaction);
    //     println!("Writing to Transactions history: {:?}", transaction);
    // }

    /// Insert a record in transaction_history
    async fn insert_transaction_in_history(
        transactions_history: SharedMap<TransactionId, TransactionRecord>,
        transaction: TransactionRecord,
    ) {
        let mut history = transactions_history
            .write()
            .await;
        println!("Transactions history before insert: {:?}", history);
        history.insert(transaction.transaction_id, transaction);
        println!("Writing to Transactions history: {:?}", transaction);
    }

    /// Processes a batch of transactions for a mixed number of clients.
    /// 1. organize the batch in a map (client vs list of records)
    /// 2. Run through the map, for each client
    ///     a. If worker count already more than max amount of workers, continue the loop
    ///     b. if no worker is spawned (check client_worker_map) then spawn `process_client_records`
    ///     for that client. Update client_worker_map to add worker as active for the client.
    ///     c. once task is done for the client, remove worker from client_work_map
    ///     
    pub async fn process_records_batch(&mut self, records: Vec<TransactionRecord>) {
        // clone shared maps
        let client_accounts = Arc::clone(&self.client_accounts);
        let transactions_history = Arc::clone(&self.transactions_history);

        // Organize the batch by client_id
        let mut client_records_map: HashMap<ClientId, Vec<TransactionRecord>> = HashMap::new();
        for record in records {
            client_records_map
                .entry(record.client_id)
                .or_insert_with(Vec::new)
                .push(record);
        }

        // Create a map to track client vs worker JoinHandle (we do not want a client to be processed
        // if there is a worker already processing it)
        let mut client_worker_map: HashMap<ClientId, JoinHandle<()>> = HashMap::new();

        // Keep looping until all client records are processed
        while !client_records_map.is_empty() {
            // list of clients being processed
            let mut clients_being_processed = vec![];

            // loop through clients_record_map, if max_workers not reached, spawn a worker to process client records
            for (client_id, client_records) in client_records_map.iter_mut() {
                // if max_workers not reached
                if client_worker_map.len() < MAX_WORKERS {
                    // Spawn a worker for the client
                    // remove the records list for the client from the map and return it as a new vector
                    // this new vector gets passed to worker
                    let records_to_process = std::mem::take(client_records);
                    let client_id_copy = *client_id; // Needed due to ownership

                    let client_accounts = Arc::clone(&self.client_accounts);
                    let transactions_history = Arc::clone(&self.transactions_history);
                    // Spawn async task for the client
                    let handle = tokio::spawn(async move {
                        TransactionProcessor::process_client_records(
                            client_id_copy,
                            records_to_process,
                            &client_accounts.clone(),
                            &transactions_history.clone(),
                        )
                        .await;
                    });

                    // Update the map to mark worker as active for this client
                    client_worker_map.insert(client_id_copy, handle);
                } else {
                    // If max workers reached, break early to wait for workers to finish
                    break;
                }

                // Remove any clients that have drained all records
                if client_records.is_empty() {
                    clients_being_processed.push(*client_id);
                }
            }

            // Clean up clients with processed records from client_records_map
            for client_id in clients_being_processed {
                client_records_map.remove(&client_id);
            }

            // Await and remove completed workers
            let mut completed_clients = vec![];
            for (client_id, handle) in client_worker_map.iter_mut() {
                if let Ok(()) = handle.await {
                    completed_clients.push(*client_id);
                }
            }

            // Remove completed clients from client_worker_map
            for client_id in completed_clients {
                client_worker_map.remove(&client_id);
            }

            // Pause briefly to allow other tasks to run
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    /// Core processing logic for client records, updating account balances and transaction history.
    /// Despotic: add to available balance
    /// Withdraw: subtract from available balance and total balance. If available balance results in
    ///     negative value then ignore the record
    /// Dispute: take from available funds the amount and add it to held funds.
    /// Resolve: Lookup the transaction and deduct amount from held funds to available funds. If
    ///     transaction does not exist, ignore the record.
    /// ChargeBack: Lookup the transaction and deduct amount from held and total balance and finally
    /// Freeze the account.
    ///
    /// Note: only update transaction history in case of successful deposit and withdraw.
    /// Because all the other transaction types we look up an existing transaction which has an
    /// amount (deposit and withdraw types are in this category)
    ///
    pub(crate) async fn process_client_records(
        client_id: ClientId,
        records: Vec<TransactionRecord>,
        client_accounts: &SharedMap<ClientId, Account>,
        transactions_history: &SharedMap<TransactionId, TransactionRecord>,
    ) {
        // Process the record and update client account
        let mut accounts = client_accounts.write().await;

        // look up the client account
        if let Some(account) = accounts.get_mut(&client_id) {
            for record in records {
                // locked account means no processing
                if account.locked {
                    println!("Account is locked, no further processing can take place!");
                    break;
                }

                match record.transaction_type {
                    TransactionType::Deposit => {
                        if let Some(amount) = record.amount {
                            account.available =
                                Self::round_to_four_decimals(account.available + amount);

                            // update transactions history
                            Self::insert_transaction_in_history(
                                transactions_history.clone(),
                                record,
                            )
                            .await;
                        }
                    }
                    TransactionType::Withdrawal => {
                        if let Some(amount) = record.amount {
                            let temp = account.available - amount;
                            if temp >= 0.0 {
                                // only update if positive or 9
                                account.available = Self::round_to_four_decimals(temp);
                                Self::insert_transaction_in_history(
                                    transactions_history.clone(),
                                    record,
                                )
                                .await;
                            } else {
                                // ignore withdrawal record, if it causes negative available
                                println!(
                                    "WARNING!!! available balance {} is \
                                        not enough for withdrawal amount {}",
                                    account.available, amount
                                );
                            }
                        }
                    }
                    TransactionType::Dispute => {
                        // look up the transaction in dispute
                        let read_record = Self::get_transaction(
                            transactions_history.clone(),
                            record.transaction_id,
                        )
                        .await;

                        println!("The transaction in dispute is  {:?}.", record);

                        if let Some(existing_record) = read_record {
                            // If the transaction exists, take from available funds the amount and add it to held funds
                            if let Some(amount) = existing_record.amount {
                                if account.available >= amount {
                                    account.available =
                                        Self::round_to_four_decimals(account.available - amount);
                                    account.held =
                                        Self::round_to_four_decimals(account.held + amount);
                                    println!(
                                        "Disputed transaction {}: amount {} taken from available and added to held.",
                                        record.transaction_id, amount
                                    );
                                } else {
                                    println!(
                                        "WARNING!!! Available balance {} is not enough for dispute amount {}",
                                        account.available, amount
                                    );
                                    // TODO return error
                                }
                            }
                        } else {
                            // If the transaction does not exist, ignore the record and continue
                            println!(
                                "WARNING!!! Transaction ID {} not found for dispute.",
                                record.transaction_id
                            );
                        }
                    }
                    TransactionType::Resolve => {
                        let read_record = Self::get_transaction(
                            transactions_history.clone(),
                            record.transaction_id,
                        )
                        .await;

                        if let Some(existing_record) = read_record {
                            if let Some(amount) = existing_record.amount {
                                if account.held >= amount {
                                    account.held =
                                        Self::round_to_four_decimals(account.held - amount);
                                    account.available =
                                        Self::round_to_four_decimals(account.available + amount);
                                    println!(
                                        "Resolved transaction {}: amount {} moved from held to available.",
                                        record.transaction_id, amount
                                    );
                                } else {
                                    println!(
                                        "WARNING!!! Held balance {} is not enough for resolve amount {}",
                                        account.held, amount
                                    );
                                }
                            }
                        } else {
                            // If the transaction does not exist, ignore the record and continue
                            println!(
                                "WARNING!!! Transaction ID {} not found for resolve.",
                                record.transaction_id
                            );
                        }
                    }
                    TransactionType::Chargeback => {
                        let read_record = Self::get_transaction(
                            transactions_history.clone(),
                            record.transaction_id,
                        )
                        .await;

                        if let Some(existing_record) = read_record {
                            if let Some(amount) = existing_record.amount {
                                account.held = Self::round_to_four_decimals(account.held - amount);
                                println!(
                                    "Chargeback for transaction {}: amount {} deducted from held and total.",
                                    record.transaction_id, amount
                                );

                                account.locked = true;
                                println!("locking account for client {}!", record.client_id);
                            }
                        } else {
                            // If the transaction does not exist, ignore the record and continue
                            println!(
                                "WARNING!!! Transaction ID {} not found for chargeback.",
                                record.transaction_id
                            );
                        }
                    }
                }
                // get total balance
                account.total = account.available + account.held;
                println!("account is : {:?}", account);
            }
        }
    }

    fn round_to_four_decimals(value: f32) -> f32 {
        (value * 10_000.0).round() / 10_000.0
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
        let records = create_test_records_simple_deposit_withdraw(client_id);

        let client_accounts: SharedMap<ClientId, Account> = Arc::new(RwLock::new(HashMap::new()));
        let transactions_history: SharedMap<TransactionId, TransactionRecord> =
            Arc::new(RwLock::new(HashMap::new()));

        // init account balance for client
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

        // Act
        TransactionProcessor::process_client_records(
            client_id,
            records.clone(),
            &client_accounts,
            &transactions_history,
        )
        .await;

        // Assert client account balances
        {
            let accounts = client_accounts.read().await;
            let account = accounts
                .get(&client_id)
                .expect("Account not found");

            assert_eq!(account.available, 4.8405);
            assert_eq!(account.held, 0.0);
            assert_eq!(account.total, 4.8405);
            assert!(!account.locked);
        }

        // Verify the transaction history
        {
            let history = transactions_history
                .read()
                .await;
            assert_eq!(history.get(&1), Some(&records[0]));
            assert_eq!(history.get(&2), Some(&records[1]));
            assert_eq!(history.get(&3), Some(&records[2]));
            assert_eq!(history.get(&4), Some(&records[3]));
        }
    }

    #[tokio::test]
    async fn test_process_client_records_dispute() {
        // Arrange

        let client_id = 1;
        let records = create_test_records_with_dispute(client_id);

        let client_accounts: SharedMap<ClientId, Account> = Arc::new(RwLock::new(HashMap::new()));
        let transactions_history: SharedMap<TransactionId, TransactionRecord> =
            Arc::new(RwLock::new(HashMap::new()));

        // init account balance for client
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

        // Act
        TransactionProcessor::process_client_records(
            client_id,
            records.clone(),
            &client_accounts,
            &transactions_history,
        )
        .await;

        // Assert client account balances
        {
            let accounts = client_accounts.read().await;
            let account = accounts
                .get(&client_id)
                .expect("Account not found");

            assert_eq!(account.available, 0.0);
            assert_eq!(account.held, 1.9234);
            assert_eq!(account.total, 1.9234);
            assert!(!account.locked);
        }

        // Verify the transaction history
        {
            let history = transactions_history
                .read()
                .await;
            assert_eq!(history.get(&1), Some(&records[0]));
        }
    }

    #[tokio::test]
    async fn test_process_client_records_resolve() {
        // Arrange

        let client_id = 1;
        let records = create_test_records_with_dispute_resolved(client_id);

        let client_accounts: SharedMap<ClientId, Account> = Arc::new(RwLock::new(HashMap::new()));
        let transactions_history: SharedMap<TransactionId, TransactionRecord> =
            Arc::new(RwLock::new(HashMap::new()));

        // init account balance for client
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

        // Act
        TransactionProcessor::process_client_records(
            client_id,
            records.clone(),
            &client_accounts,
            &transactions_history,
        )
        .await;

        // Assert client account balances
        {
            let accounts = client_accounts.read().await;
            let account = accounts
                .get(&client_id)
                .expect("Account not found");

            assert_eq!(account.available, 1.9234);
            assert_eq!(account.held, 0.0);
            assert_eq!(account.total, 1.9234);
            assert!(!account.locked);
        }

        // Verify the transaction history
        {
            let history = transactions_history
                .read()
                .await;
            assert_eq!(history.get(&1), Some(&records[0]));
        }
    }

    #[tokio::test]
    async fn test_process_client_records_chargeback() {
        // Arrange

        let client_id = 1;
        let records = create_test_records_with_dispute_chargeback(client_id);

        let client_accounts: SharedMap<ClientId, Account> = Arc::new(RwLock::new(HashMap::new()));
        let transactions_history: SharedMap<TransactionId, TransactionRecord> =
            Arc::new(RwLock::new(HashMap::new()));

        // init account balance for client
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

        // Act
        TransactionProcessor::process_client_records(
            client_id,
            records.clone(),
            &client_accounts,
            &transactions_history,
        )
        .await;

        // Assert client account balances
        {
            let accounts = client_accounts.read().await;
            let account = accounts
                .get(&client_id)
                .expect("Account not found");

            assert_eq!(account.available, 200.023);
            assert_eq!(account.held, 0.0);
            assert_eq!(account.total, 200.023);
            assert!(account.locked);
        }

        // Verify the transaction history
        {
            let history = transactions_history
                .read()
                .await;
            assert_eq!(history.get(&1), Some(&records[0]));
        }
    }

    #[tokio::test]
    async fn test_process_client_batch_records_deposit_withdrawal() {
        // Arrange
        let client_id_1 = 1;
        let client_id_2 = 2;
        let client_id_3 = 3;
        let client_id_4 = 4;
        let transaction_id_1 = 1;
        let transaction_id_2 = 2;
        let transaction_id_3 = 3;
        let transaction_id_4 = 4;
        let transaction_id_5 = 5;

        let records = create_test_records_with_mixed_clients_deposits_withdrawals();

        let client_accounts: SharedMap<ClientId, Account> = Arc::new(RwLock::new(HashMap::new()));
        let transactions_history: SharedMap<TransactionId, TransactionRecord> =
            Arc::new(RwLock::new(HashMap::new()));

        let mut processor = TransactionProcessor {
            client_accounts: client_accounts.clone(),
            transactions_history: transactions_history.clone(),
        };

        // init account balance for client
        {
            let mut accounts = client_accounts.write().await;
            for client_id in 1..=4 {
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
        }

        // Act
        processor
            .process_records_batch(records.clone())
            .await;

        // Assert client account balances
        {
            let accounts = client_accounts.read().await;
            let account = accounts
                .get(&client_id_1)
                .expect("Account not found");

            assert_eq!(account.available, 1.9234);
            assert_eq!(account.held, 0.0);
            assert_eq!(account.total, 1.9234);
            assert!(!account.locked);
        }

        // Verify the transaction history
        {
            let history = transactions_history
                .read()
                .await;
            assert_eq!(history.get(&transaction_id_1), Some(&records[0]));
        }
    }

    fn create_test_records_simple_deposit_withdraw(client_id: ClientId) -> Vec<TransactionRecord> {
        let transaction_id_1 = 1;
        let transaction_id_2 = 2;
        let transaction_id_3 = 3;
        let transaction_id_4 = 4;

        let record1 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id,
            transaction_id: transaction_id_1,
            amount: Some(1.9234),
        };
        let record2 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id,
            transaction_id: transaction_id_2,
            amount: Some(23.3525),
        };
        let record3 = TransactionRecord {
            transaction_type: TransactionType::Withdrawal,
            client_id,
            transaction_id: transaction_id_3,
            amount: Some(50.234),
        };
        let record4 = TransactionRecord {
            transaction_type: TransactionType::Withdrawal,
            client_id,
            transaction_id: transaction_id_4,
            amount: Some(20.4354),
        };

        vec![record1, record2, record3, record4]
    }

    fn create_test_records_with_dispute(client_id: ClientId) -> Vec<TransactionRecord> {
        let transaction_id_1 = 1;

        let record1 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id,
            transaction_id: transaction_id_1,
            amount: Some(1.9234),
        };
        let record2 = TransactionRecord {
            transaction_type: TransactionType::Dispute,
            client_id,
            transaction_id: transaction_id_1,
            amount: None,
        };

        vec![record1, record2]
    }

    fn create_test_records_with_dispute_resolved(client_id: ClientId) -> Vec<TransactionRecord> {
        let transaction_id_1 = 1;

        let record1 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id,
            transaction_id: transaction_id_1,
            amount: Some(1.9234),
        };
        let record2 = TransactionRecord {
            transaction_type: TransactionType::Dispute,
            client_id,
            transaction_id: transaction_id_1,
            amount: None,
        };
        let record3 = TransactionRecord {
            transaction_type: TransactionType::Resolve,
            client_id,
            transaction_id: transaction_id_1,
            amount: None,
        };

        vec![record1, record2, record3]
    }

    fn create_test_records_with_dispute_chargeback(client_id: ClientId) -> Vec<TransactionRecord> {
        let transaction_id_1 = 1;
        let transaction_id_2 = 2;
        let transaction_id_3 = 3;
        let transaction_id_4 = 4;

        let record1 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id,
            transaction_id: transaction_id_1,
            amount: Some(1.9234),
        };
        let record2 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id,
            transaction_id: transaction_id_2,
            amount: Some(200.023),
        };
        let record3 = TransactionRecord {
            transaction_type: TransactionType::Dispute,
            client_id,
            transaction_id: transaction_id_1,
            amount: None,
        };
        let record4 = TransactionRecord {
            transaction_type: TransactionType::Chargeback,
            client_id,
            transaction_id: transaction_id_1,
            amount: None,
        };

        // transactions that should be ignored because of prev chargeback freezes the account
        let record5 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id,
            transaction_id: transaction_id_3,
            amount: Some(2300.00),
        };
        let record6 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id,
            transaction_id: transaction_id_4,
            amount: Some(12.12),
        };

        vec![record1, record2, record3, record4, record5, record6]
    }

    fn create_test_records_with_mixed_clients_deposits_withdrawals() -> Vec<TransactionRecord> {
        let client_id_1 = 1;
        let client_id_2 = 2;
        let client_id_3 = 3;
        let client_id_4 = 4;
        let transaction_id_1 = 1;
        let transaction_id_2 = 2;
        let transaction_id_3 = 3;
        let transaction_id_4 = 4;
        let transaction_id_5 = 5;

        let record1 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id: client_id_1,
            transaction_id: transaction_id_1,
            amount: Some(1.9234),
        };
        let record2 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id: client_id_2,
            transaction_id: transaction_id_2,
            amount: Some(200.023),
        };
        let record3 = TransactionRecord {
            transaction_type: TransactionType::Withdrawal,
            client_id: client_id_2,
            transaction_id: transaction_id_3,
            amount: Some(10.00),
        };
        let record4 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id: client_id_3,
            transaction_id: transaction_id_4,
            amount: Some(1.1245),
        };
        let record5 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id: client_id_4,
            transaction_id: transaction_id_5,
            amount: Some(1.1245),
        };

        vec![record1, record2, record3, record4, record5]
    }
}
