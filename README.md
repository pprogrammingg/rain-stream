# rain-stream

This is a hobby project for ingesting and processing large and concurrent streams of data from CSV files.
In this example, CSV data represent transactions such as deposit, withdraw, dispute, etc being done against
client accounts.

# Performance Results

Parameters hardcoded in app:

```rust
const CSV_RECORDS_CHUNK_SIZE: usize = 1000; // CSV batch read at once
const MAX_WORKERS: usize = 4; // one worker per client is spawned in TransactionProcessor
```

| # of Records | Time Elapsed (ms) |
|--------------|-------------------|
| 38           | 12.633834ms       |
| 10_0000      | 226.5ms           |
| 100_000      | 2871ms            |
| 1_000_000    | 28478ms           |

```rust
const CSV_RECORDS_CHUNK_SIZE: usize = 10_000; // CSV batch read at once
const MAX_WORKERS: usize = 4; // one worker per client is spawned in TransactionProcessor
```

| # of Records | Time Elapsed (ms) |
|--------------|-------------------|
| 38           | 14.74ms           |
| 10_0000      | 80.64ms           |
| 100_000      | 643ms             |
| 1_000_000    | 6000ms            |

Very reasonably seems like increasing `CSV_RECORS_CHUNK_SIZE` 10 folds, reduces times by around 4 or 5 times.
Furthermore, increasing `MAX_WORKERS` from 4 to 80 for 1_000_000 records made no significant difference (this is
most likely because this is because performance test data was only based on 4 clients.

# Design Decisions

## Iteration 1

The tx engine does few major functions:

- Read CSV one record at the time since order of rows is assumed to represent chronological orde).
- Each record is deserialized and put in an incoming tx queue for a specific client.
- A notification system notifies the system to spawn a tx_processor task when client queue receives incoming tx.
- Final result of processing is persisted in `balances` table of a database, with a transaction history keeping record
  of processed tx.
- Output will be the query of `balances` table to stdout.

### CSV Reader

To make reading CSVs efficient, consider:

- should we load whole CSV to memory, read record by record, or N records at the time: some research shows that CSV
  crate internally handles efficient reading, however reading one record at the time or N records explicitely might be
  the
  right approach. the idea is that reader is an independent task (spawned or just using the main process) to keep
  reading
  without need to finish for tx processing to take place.

When 1 or bunch of records are read, they are put in client specific in-memory queues so to have tx_processor  
process those asynchronously.

### Client Incoming Transactions Queues

- Maintains thread-safe map of `client_id` vs `list of tx_records`
- new client specific records read by csv reader are added to list of tx_records based on client
- a notification is issued when txs are inserted in the queue

### TX Processing

- Main loop listens for notification from Client Queue manager
- Based on notification checks a tracking map to see whether a processor already running for the client. If not spawn
  one.
- Tx processor spawns and takes first N txs from the client queue for processing
- in the batch of N txs, if tx is deposit or withdraw type simply do an intial balance read for the client, and
  add and subtract balances (should negative balance occur, ignore last withdraw). Once the next tx encountered is not
  deposit or withdraw type, then commit the new valid balance.
- If tx is dispute, resolve or chargeback (for which DB look up is needed) lookup the record and make appropriate
  further action per requirement.
- once done with all the processing, take the next N txs from client queue
- When no more txs exist, remove self from `cleint_tx_processor_tracking_map`.

## Iteration 2

After implementing iteration 1, it seems as though CSV crate reading records one by one, successfuly enqueues the
records
in the shared queue, and sends notifs to TransactionsManager to process the task. However, it turns out that this setup
somehow waits only CSV reading is finished and then start switching to record processing.

The design will now change with the following improvements:

1. CSV Reader will read not just 1 record at the time but a BATCH_SIZE (defined at app level) number of reords are read
   before processing them.
2. The batch of records then are passed to a `batch_records_processor`. This method will divide the batch of records
   per `client_id` and for each `client_id` spawns a new task to handle the records for that client.

# Iteration 1 Tasks

- [x] An integration test that always passes and `hello world` is printed by calling hello_word method of app
    - [x] scaffold basic project with code fmt tool and settings in IDE updated
    - [x] run an integration tests that call `hello_world` method
    - [x] create Git CI build flow and make sure it passes and runs the integration test

- [x] CSV Record Reading and Queueing: An integration test that ingest a CSV file contains the following CSV data

    ```
        type, client, tx, amount
        deposit, 1, 1, 1.0
        deposit, 2, 2, 2.0
        deposit, 1, 3, 2.0
        withdrawal, 1, 4, 1.5
        withdrawal, 2, 5, 3.0
    ```


- [x] Sub-tasks:
    - [x] Add helper function to write CSV. This will be used during each test to write a CSV.
    - [x] In arrange phase of the test create a CSV with name "basic_read.csv". Delete the file after reading is done.
    - [x] Once read the output object containing parsed CSV data should match what the input data gave
    - [x] Efficiency: make CSV reading an independent task spawned so other processes do not have to wait for it
    - [x] Deserialize read records to a Rust tx record type with field types per requirement
    - [x] create client queue manager module, rows read in csv are inserted in client specific incoming tx queues

# Iteration 2 Tasks

- [ ] TransactionProcessor module will encapsulate (
    - [x] `client_accounts`, a thread-safe hashmap containing `client_id` vs `Account` (Account consists of
      available, held, total, locked fields)
    - [x] `transactions_history`, a thread-safe hashmap containing `client_id` vs `TransactionRecord`, used for lookups
      needed per `resolve` and `charge_back` transaction types
    - [x] (1 hr) method `insert_or_update_client_account(client_id, transaction_record)` + unit test
    - [x] (1 hr) method `get_transaction(transaction_id)` + unit test
    - [ ] (1 hr) method `process_records_batch(Vec<TransactionRecord>)` which spins a worker per client to do
      `process_client_records(Vec<TransactionRecord>)`  (`MAX_WORKERS` = 10 by default)
    - [x] (2 hrs ) method `process_client_records(Vec<TransactionRecord>)` (this is what is passed to `spawn_task`
      from `process_records_batch(Vec<TransactionRecord>)` per client) + unit tests. This is the core of balance
      calculation
      engine (take care of correct decimals, math safety, lookups, etc)
        - unit tests for deposit, withdraw (withdraw ignored case as well), dispute, resolve and chargeback (account
          freeze
          causes other txs to be ignored)

- [ ] Csv Reader Modifications
    - [ ] (1 hr) Read in a batches and When sufficient batch size is reached spawn a task for
      `process_records_batch(Vec<TransactionRecord>` )

- [ ] Functionality tests
    - [x] (1 hr) Utility: make the utility function to create CSV of N records (later we use this for performance test
      as well, we need 2B eventually)
    - [x] (1 hr) Assertion Util create expected `Vec<TransactionRecrods>`, read output.csv into
      `Vec<TransactionRecords>` and compare
        - Tests:
            - [x] (1 hr) when clients contains valid withdrawal, deposit, dispute and resolve 7 client 1 , 4 client 2, 3
              client 3 records
            - [x] (1 hr) when client account is locked 7 client 1 (record 4 is a chargeback), 10 client 2 (record 5 is a
              chargeback), 3 client 3 records
            - [x] (1 hr) when client withdrawal is more than available balance, 5 client 1 (some withdrawals resulting
              in more than balance)
            - [x] (1 hr) when client records deposit, withdrawal, and in between contains dispute, resolve, dispute,
              chargeback variation and some more records

# Performance Tuning and Results

- [x] Performance (few chargebacks towards the end) (2 hrs - 4 hrs)
    - [x] 10k txs
    - [x] 100k txs
    - [x] 1M txs
