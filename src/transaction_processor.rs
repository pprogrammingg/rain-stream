use std::{
    collections::HashMap,
    sync::Arc,
};

use tokio::{
    sync::RwLock,
    task::JoinHandle,
};

use crate::domain::{
    TransactionRecord,
    TransactionType,
};

pub type ClientId = u16;
pub type TransactionId = u32;
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Account {
    pub available: f32,
    pub held: f32,
    pub total: f32,
    pub locked: bool,
}

pub type SharedMap<K, V> = Arc<RwLock<HashMap<K, V>>>;

pub struct TransactionProcessor {}

const MAX_WORKERS: usize = 4;

impl TransactionProcessor {
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
    async fn insert_transaction_in_history(
        transactions_history: SharedMap<TransactionId, TransactionRecord>,
        transaction: TransactionRecord,
    ) {
        let mut history = transactions_history
            .write()
            .await;
        // println!("Transactions history before insert: {:?}", history);
        history.insert(transaction.transaction_id, transaction);
        // println!("Writing to Transactions history: {:?}", transaction);
    }

    /// Processes a batch of transactions for a mixed number of clients.
    /// 1. organize the batch in a map (client vs list of records)
    /// 2. Run through the map, for each client a. If worker count already more than max amount of
    ///    workers, continue the loop b. if no worker is spawned (check client_worker_map) then
    ///    spawn `process_client_records` do not process the client unless its associated task is
    ///    done. c. once task is done for the client, remove worker from client_work_map
    pub async fn process_records_batch(
        records: Vec<TransactionRecord>,
        client_accounts: SharedMap<ClientId, Account>,
        transactions_history: SharedMap<TransactionId, TransactionRecord>,
        client_worker_map: SharedMap<ClientId, JoinHandle<()>>,
    ) {
        // Organize the batch by client_id
        let mut client_records_map: HashMap<ClientId, Vec<TransactionRecord>> = HashMap::new();
        for record in records {
            client_records_map
                .entry(record.client_id)
                .or_default()
                .push(record);
        }

        // println!("client_records_map is {:?}", client_records_map);
        // println!("================================");

        // Keep looping until all client records are processed
        while !client_records_map.is_empty() {
            let mut clients_being_processed = vec![];

            // Iterate through client_records_map to spawn tasks
            for (client_id, client_records) in client_records_map.iter_mut() {
                // Check if client already has an active task, if so continue
                {
                    let client_worker_map_read = client_worker_map.read().await;
                    if client_worker_map_read.contains_key(client_id) {
                        // println!("Client {} already has an active task", client_id);
                        continue;
                    }
                }

                // Check if max worker limit has been reached, if so continue
                {
                    let client_worker_map_read = client_worker_map.read().await;
                    if client_worker_map_read.len() >= MAX_WORKERS {
                        println!("Max worker count reached; waiting for workers to finish.");
                        continue;
                    }
                }

                let records_to_process = std::mem::take(client_records);
                let client_id_copy = *client_id;

                let client_accounts = Arc::clone(&client_accounts);
                let transactions_history = Arc::clone(&transactions_history);

                // Spawn async task for the client
                let handle = tokio::spawn(async move {
                    TransactionProcessor::process_client_records(
                        client_id_copy,
                        records_to_process,
                        &client_accounts,
                        &transactions_history,
                    )
                    .await;
                });

                // update client_worker_map with the handle
                {
                    let mut client_worker_map_write = client_worker_map
                        .write()
                        .await;
                    client_worker_map_write.insert(client_id_copy, handle);
                    // println!("client_worker_map: {:?}", client_worker_map_write);
                    // println!("===========================================");
                }

                // Push client to clients_being_processed list
                if client_records.is_empty() {
                    clients_being_processed.push(*client_id);
                }
            }

            // Important! Remove processed clients from client_records_map (otherwise we keep
            // processing)
            for client_id in clients_being_processed {
                client_records_map.remove(&client_id);
            }

            // list of complete_clients
            let mut completed_clients = vec![];

            // Await handles in client_worker_map to complete
            {
                let mut client_worker_map_write = client_worker_map
                    .write()
                    .await;
                for (client_id, handle) in client_worker_map_write.iter_mut() {
                    if handle.await.is_ok() {
                        // println!("Completed task for client_id {}", client_id);
                        // println!("===========================================");
                        completed_clients.push(*client_id);
                    }
                }

                // Clean up completed clients from client_worker_map
                for client_id in completed_clients {
                    // println!("Removing client_id {} from client_worker_map", client_id);
                    // println!("===========================================");
                    client_worker_map_write.remove(&client_id);
                }
            }
        }
        // Small sleep to prevent tight looping
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
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
    pub(crate) async fn process_client_records(
        client_id: ClientId,
        records: Vec<TransactionRecord>,
        client_accounts: &SharedMap<ClientId, Account>,
        transactions_history: &SharedMap<TransactionId, TransactionRecord>,
    ) {
        // Ensure an account is inserted for the client with 0 balances if not already
        TransactionProcessor::ensure_client_account(client_id, Arc::clone(client_accounts)).await;
        {
            // Process the record and update client account
            let mut accounts = client_accounts.write().await;

            // look up the client account
            if let Some(account) = accounts.get_mut(&client_id) {
                for record in records {
                    // locked account means no processing
                    if account.locked {
                        // println!("Account is locked, no further processing can take place!");
                        // println!("******************************************");
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
                                    // println!(
                                    //     "WARNING!!! available balance {} is \
                                    //     not enough for withdrawal amount {}",
                                    //     account.available, amount
                                    // );
                                    // println!("******************************************");
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

                            // println!("The transaction in dispute is  {:?}.", record);
                            // println!("******************************************");

                            if let Some(existing_record) = read_record {
                                // If the transaction exists, take from available funds the amount
                                // and add it to held funds
                                if let Some(amount) = existing_record.amount {
                                    if account.available >= amount {
                                        account.available = Self::round_to_four_decimals(
                                            account.available - amount,
                                        );
                                        account.held =
                                            Self::round_to_four_decimals(account.held + amount);
                                        // println!(
                                        //     "Disputed transaction {}: amount {} taken from
                                        // available and added to held.",
                                        //     record.transaction_id, amount
                                        // );
                                        // println!("******************************************");
                                    } else {
                                        // ignore
                                        // println!(
                                        //     "WARNING!!! Available balance {} is not enough for
                                        // dispute amount {}",
                                        //     account.available, amount
                                        // );
                                        // println!("******************************************");
                                    }
                                }
                            } else {
                                // If the transaction does not exist, ignore the record and continue
                                // println!(
                                //     "WARNING!!! Transaction ID {} not found for dispute.",
                                //     record.transaction_id
                                // );
                                // println!("******************************************");
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
                                        account.available = Self::round_to_four_decimals(
                                            account.available + amount,
                                        );
                                        // println!(
                                        //     "Resolved transaction {}: amount {} moved from held
                                        // to available.",
                                        //     record.transaction_id, amount
                                        // );
                                        // println!("******************************************");
                                    } else {
                                        // ignore
                                        //     println!(
                                        //         "WARNING!!! Held balance {} is not enough for
                                        // resolve amount {}",
                                        //         account.held, amount
                                        //     );
                                        //     println!("******************************************"
                                        // );
                                    }
                                }
                            } else {
                                // If the transaction does not exist, ignore the record and continue
                                // println!(
                                //     "WARNING!!! Transaction ID {} not found for resolve.",
                                //     record.transaction_id
                                // );
                                // println!("******************************************");
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
                                    account.held =
                                        Self::round_to_four_decimals(account.held - amount);
                                    // println!(
                                    //     "Chargeback for transaction {}: amount {} deducted from
                                    // held and total.",
                                    //     record.transaction_id, amount
                                    // );
                                    // println!("******************************************");

                                    account.locked = true;
                                    // println!("locking account for client {}!", record.client_id);
                                    // println!("******************************************");
                                }
                            } else {
                                // ignore
                                // If the transaction does not exist, ignore the record and continue
                                // println!(
                                //     "WARNING!!! Transaction ID {} not found for chargeback.",
                                //     record.transaction_id
                                // );
                                // println!("******************************************");
                            }
                        }
                    }
                    // get total balance
                    account.total = account.available + account.held;
                    // println!("account is : {:?}", account);
                    // println!("******************************************");
                }
            }
        }

        // println!("Exiting worker task for client_id {}", client_id);
        // println!("******************************************");
    }

    async fn ensure_client_account(
        client_id: ClientId,
        client_accounts: SharedMap<ClientId, Account>,
    ) {
        // Insert a default record if client account does not exist
        let mut accounts = client_accounts.write().await;
        accounts
            .entry(client_id)
            .or_insert_with(|| Account {
                available: 0.0,
                held: 0.0,
                total: 0.0,
                locked: false,
            });
    }

    fn round_to_four_decimals(value: f32) -> f32 {
        (value * 10_000.0).round() / 10_000.0
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::Arc,
    };

    use tokio::sync::RwLock;

    use super::*;

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
        let records = create_test_records_batch();

        let client_accounts: SharedMap<ClientId, Account> = Arc::new(RwLock::new(HashMap::new()));
        let transactions_history: SharedMap<TransactionId, TransactionRecord> =
            Arc::new(RwLock::new(HashMap::new()));
        let client_worker_map: SharedMap<ClientId, JoinHandle<()>> =
            Arc::new(RwLock::new(HashMap::new()));

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
        TransactionProcessor::process_records_batch(
            records.clone(),
            client_accounts.clone(),
            transactions_history.clone(),
            client_worker_map,
        )
        .await;

        //Assert client account balances
        {
            let accounts = client_accounts.read().await;

            // expected
            let expected_account_1 = Some(&Account {
                available: 400.0,
                held: 0.0,
                total: 400.0,
                locked: false,
            });

            let expected_account_2 = Some(&Account {
                available: 400.0,
                held: 0.0,
                total: 400.0,
                locked: false,
            });

            let expected_account_3 = Some(&Account {
                available: 350.0,
                held: 0.0,
                total: 350.0,
                locked: false,
            });

            let expected_account_4 = Some(&Account {
                available: 800.0,
                held: 0.0,
                total: 800.0,
                locked: true,
            });

            assert_eq!(
                accounts.get(&1),
                expected_account_1,
                "client_1 account does not match"
            );
            assert_eq!(
                accounts.get(&2),
                expected_account_2,
                "client_2 account does not match"
            );
            assert_eq!(
                accounts.get(&3),
                expected_account_3,
                "client_3 account does not match"
            );
            assert_eq!(
                accounts.get(&4),
                expected_account_4,
                "client_4 account does not match"
            );
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

    fn create_test_records_batch() -> Vec<TransactionRecord> {
        let mut records = Vec::new();
        let mut transaction_id = 1;

        // Create 5 deposits for clients 1 and 2
        for client_id in 1..=2 {
            for _ in 0..5 {
                records.push(TransactionRecord {
                    transaction_type: TransactionType::Deposit,
                    client_id,
                    transaction_id,
                    amount: Some(100.0),
                });
                transaction_id += 1;
            }
        }

        // Create 2 withdrawals for clients 1 and 2
        for client_id in 1..=2 {
            for _ in 0..2 {
                records.push(TransactionRecord {
                    transaction_type: TransactionType::Withdrawal,
                    client_id,
                    transaction_id,
                    amount: Some(50.0),
                });
                transaction_id += 1;
            }
        }

        // Create very large amount of records for client 3
        for _ in 0..7 {
            records.push(TransactionRecord {
                transaction_type: TransactionType::Deposit,
                client_id: 3,
                transaction_id,
                amount: Some(50.0),
            });
            transaction_id += 1;
        }

        // dispute a transaction for client_3
        let disputed_transaction = transaction_id - 2;
        records.push(TransactionRecord {
            transaction_type: TransactionType::Dispute,
            client_id: 3,
            transaction_id: disputed_transaction,
            amount: None,
        });

        // resolve the transaction for client_3
        let disputed_transaction = transaction_id - 2;
        records.push(TransactionRecord {
            transaction_type: TransactionType::Resolve,
            client_id: 3,
            transaction_id: disputed_transaction,
            amount: None,
        });

        // Create 5 records for client 4
        for _ in 0..5 {
            records.push(TransactionRecord {
                transaction_type: TransactionType::Deposit,
                client_id: 4,
                transaction_id,
                amount: Some(200.0),
            });
            transaction_id += 1;
        }

        // dispute a transaction in client 4
        let disputed_transaction = transaction_id - 2;

        records.push(TransactionRecord {
            transaction_type: TransactionType::Dispute,
            client_id: 4,
            transaction_id: disputed_transaction,
            amount: None,
        });

        // charge back the disputed transaction
        records.push(TransactionRecord {
            transaction_type: TransactionType::Chargeback,
            client_id: 4,
            transaction_id: disputed_transaction,
            amount: None,
        });

        // Further transactions for 4 are ignored
        records.push(TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id: 4,
            transaction_id,
            amount: Some(10000.0),
        });

        records
    }
}
