use crate::TransactionsQueue;
use std::sync::Arc;
use std::time::Duration;
use tokio::task;

pub struct TransactionsManager {
    transaction_queue: Arc<TransactionsQueue>,
}

impl TransactionsManager {
    /// Initialize the `TransactionManager` with a shared `TransactionQueue`.
    /// Starts an async loop that listens for incoming client transactions.
    pub fn start() -> Self {
        let transaction_queue = Arc::new(TransactionsQueue::new());
        let manager = TransactionsManager {
            transaction_queue: Arc::clone(&transaction_queue),
        };

        // Start listening for incoming transactions specific to clients
        manager.start_listening_for_transactions();

        manager
    }

    /// listen for incoming client transactions.
    fn start_listening_for_transactions(&self) {
        let queue = Arc::clone(&self.transaction_queue);

        task::spawn(async move {
            loop {
                // Simulate listening for transactions with a delay
                tokio::time::sleep(Duration::from_secs(5)).await;
                println!("Listening for incoming transactions...");

                // Here you could add logic to fetch/process transactions
                // For example, dequeue transactions from clients and process them
                // queue.dequeue_for_processing();
            }
        });
    }

    /// Returns a clone of the shared `TransactionQueue`
    pub fn get_transaction_queue(&self) -> Arc<TransactionsQueue> {
        Arc::clone(&self.transaction_queue)
    }
}
