use crate::domain::TransactionRecord;
use std::collections::HashMap;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug)]
pub struct TransactionsQueue {
    clients_incoming_tx_queue: Mutex<HashMap<u16, Vec<TransactionRecord>>>,
}

impl TransactionsQueue {
    ///
    /// Create TransactionQueue by instantiating clients_incoming_tx_queue and a msg_sender.
    /// msg_sender is used in enqueue to notify `TransactionsManager.listen_for_incoming_tx`
    /// to handle transactions specific to client in the queue
    /// Return the queue and msg_receiver
    ///
    pub fn new() -> Self {
        TransactionsQueue {
            clients_incoming_tx_queue: Mutex::new(HashMap::new()),
        }
    }

    ///
    /// 1. Enqueue a transaction for a specific client
    /// 2. Notify transaction manager that client received a transaction
    ///
    pub async fn enqueue(
        &self,
        client_id: u16,
        tx: TransactionRecord,
        msg_sender: mpsc::Sender<u16>,
    ) {
        let mut queue = self
            .clients_incoming_tx_queue
            .lock()
            .await;

        queue
            .entry(client_id)
            .or_default()
            .push(tx);

        println!("Enqueued transaction for client {}: {:?}", client_id, tx);

        // notify TransactionsManager that client transaction has arrived
        msg_sender
            .send(client_id)
            .await
            .expect("Could not send notification!");
    }

    // Remove entry for client_id
    pub async fn remove_client_from_queue(&self, client_id: &u16) {
        let mut queue = self
            .clients_incoming_tx_queue
            .lock()
            .await;

        queue.remove(client_id);
        println!("Removed client {} from queue", client_id);
    }

    // Get transaction list for a specific client
    pub async fn get_tx_list_for_client(&self, client_id: &u16) -> Option<Vec<TransactionRecord>> {
        let queue = self
            .clients_incoming_tx_queue
            .lock()
            .await;

        queue.get(&client_id).cloned()
    }

    // Print the clients' incoming transaction queue
    pub async fn print_queue(&self) {
        let queue = self
            .clients_incoming_tx_queue
            .lock()
            .await;
        println!("clients_incoming_tx_queue is \n{:?}", queue);
    }
}
