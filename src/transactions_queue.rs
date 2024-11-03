use crate::domain::TransactionType;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct TransactionsQueue {
    clients_incoming_tx_queue: RwLock<HashMap<String, Vec<TransactionType>>>,
}

impl TransactionsQueue {
    pub fn new() -> Self {
        TransactionsQueue {
            clients_incoming_tx_queue: RwLock::new(HashMap::new()),
        }
    }

    // Enqueue a transaction for a specific client
    pub fn enqueue(&self, client_id: String, tx: TransactionType) {
        let mut queue = self
            .clients_incoming_tx_queue
            .write()
            .unwrap();

        queue
            .entry(client_id.clone())
            .or_default()
            .push(tx);

        println!("Enqueued transaction for client {}: {:?}", client_id, tx);
    }

    // Remove entry for client_id
    pub fn remove_client_from_queue(&self, client_id: &str) {
        let mut queue = self
            .clients_incoming_tx_queue
            .write()
            .unwrap();

        queue.remove(client_id);
        println!("Removed client {} from queue", client_id);
    }

    // Get transaction list for a specific client
    pub fn get_tx_list_for_client(&self, client_id: String) -> Option<Vec<TransactionType>> {
        let queue = self
            .clients_incoming_tx_queue
            .read()
            .unwrap();
        queue.get(&client_id).cloned()
    }

    // Print the clients' incoming transaction queue
    pub fn print_queue(&self) {
        let queue = self
            .clients_incoming_tx_queue
            .read()
            .unwrap();
        println!("clients_incoming_tx_queue is \n{:?}", queue);
    }
}
