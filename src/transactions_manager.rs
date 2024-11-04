use crate::TransactionsQueue;
use std::sync::Arc;
use tokio::sync::mpsc;

const MSG_CHANNEL_SIZE: usize = 1024;

pub struct TransactionsManager {
    pub queue: Arc<TransactionsQueue>,
    pub sender: mpsc::Sender<u16>,
    pub receiver: mpsc::Receiver<u16>,
}

impl TransactionsManager {
    pub fn new() -> Self {
        // 1.
        let queue = Arc::new(TransactionsQueue::new());
        let (sender, receiver) = mpsc::channel::<u16>(MSG_CHANNEL_SIZE);

        TransactionsManager {
            queue,
            sender,
            receiver,
        }
    }
}

///
/// listen for incoming client transactions in a loop
/// if a worker is spawned already for client's transactions then do not do anything
/// else spawn a worker if MAX_WORKER limit not reached.
/// If MAX_WORKER limit is reached, do nothing and go back to the beginning of the loop.
///
pub async fn listen_for_transactions(
    transactions_queue: Arc<TransactionsQueue>,
    mut receiver: mpsc::Receiver<u16>,
) {
    println!("Start listening for transactions...");
    loop {
        // Listen for notifications
        if let Some(client_id) = receiver.recv().await {
            println!("Received notification for client: {}", client_id);
            // Process the notification
            if let Some(transactions) = transactions_queue
                .get_tx_list_for_client(&client_id)
                .await
            {
                println!(
                    "Processing transactions for client {}: {:?}",
                    client_id, transactions
                );
            } else {
                println!("Client {} has an empty queue", client_id);
            }
        }
    }
}
