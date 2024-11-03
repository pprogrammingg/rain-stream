# rain-stream

This is a hobby project for ingesting and processing large and concurrent streams of data from CSV files.
In this example, CSV data represent transactions such as deposit, withdraw, dispute, etc being done against
client accounts.

# Design Decisions

## Overview

The tx engine does few major functions:

- Read CSV one record at the time since order of rows is assumed to represent chronological orde).
- Each record is deserialized and put in an incoming tx queue for a specific client.
- A notification system notifies the system to spawn a tx_processor task when client queue receives incoming tx.
- Final result of processing is persisted in `balances` table of a database, with a transaction history keeping record
  of processed tx.
- Output will be the query of `balances` table to stdout.

## CSV Reader

To make reading CSVs efficient, consider:

- should we load whole CSV to memory, read record by record, or N records at the time: some research shows that CSV
  crate internally handles efficient reading, however reading one record at the time or N records explicitely might be
  the
  right approach. the idea is that reader is an independent task (spawned or just using the main process) to keep
  reading
  without need to finish for tx processing to take place.

When 1 or bunch of records are read, they are put in client specific in-memory queues so to have tx_processor  
process those asynchronously.

## Client Incoming Transactions Queues

- Maintains thread-safe map of `client_id` vs `list of tx_records`
- new client specific records read by csv reader are added to list of tx_records based on client
- a notification is issued when txs are inserted in the queue

## TX Processing

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

# Tasks Breakdown

- [x] An integration test that always passes and `hello world` is printed by calling hello_word method of app
    - [x] scaffold basic project with code fmt tool and settings in IDE updated
    - [x] run an integration tests that call `hello_world` method
    - [x] create Git CI build flow and make sure it passes and runs the integration test

- [ ] CSV Record Reading and Queueing: An integration test that ingest a CSV file contains the following CSV data

    ```
        type, client, tx, amount
        deposit, 1, 1, 1.0
        deposit, 2, 2, 2.0
        deposit, 1, 3, 2.0
        withdrawal, 1, 4, 1.5
        withdrawal, 2, 5, 3.0
    ```


- [ ] Sub-tasks:
    - [x] Add helper function to write CSV. This will be used during each test to write a CSV.
    - [x] In arrange phase of the test create a CSV with name "basic_read.csv". Delete the file after reading is done.
    - [x] Once read the output object containing parsed CSV data should match what the input data gave
    - [x] Efficiency: make CSV reading an independent task spawned so other processes do not have to wait for it
    - [x] Deserialize read records to a Rust tx record type with field types per requirement
    - [ ] create client queue manager module, rows read in csv are inserted in client specific incoming tx queues
    - [ ] write unit test to check rows read are separated and put in the right queues
    - [ ] integration test when no input file is provided in the CLI
    - [ ] integration test when file path of CSV is invalid
    - [ ] integration test when data can not be parsed in to the proper object


- [ ] Database: bring a postgres database with tables hold account and processed transactions
    - [ ] dockerize the app such that running the app brings up db in docker, creates or updates tables based on
      migration

expected result:

- [ ] Task processor with database : a task which takes batches of txs from client queues and processes them.
    - [ ] Notification system wakes up a task to work on a certain client queue (as such need to track whether a task
      for
      for the client already exist if not spawn one) - use MPSC for notif
    - [ ] Integration test where CSV containing multiple clients is passed to program. Data for CSV:
        ```
            type, client, tx, amount
            deposit, 1, 1, 1.0
            deposit, 2, 2, 2.0
            deposit, 1, 3, 2.0
            withdrawal, 1, 4, 1.5
            withdrawal, 2, 5, 3.0
            dispute, 1, 1
            resolve, 1, 1
            dispute, 2, 2
            chargeback, 1, 2
        ```
      expected result:

        ```
            client, available, held, total, locked
            1, 1.5, 0.0, 1.5, false
            2, 0.0, 0.0, 0.0, true
        ```

Also, Error "Withdrawal cannot proceed due to insufficient available funds" should display logs regarding client 2 tx 5
but should not crash app

- [ ] Other integration tests:
    - [ ] When client 2 account is locked before another tx.
        ```
            type, client, tx, amount
            deposit, 1, 1, 1.034534634
            deposit, 2, 2, 2.0313
            dispute, 2, 2
            chargeback, 2, 2
            deposit, 1, 3, 2.053252345
            withdrawal, 1, 4, 1.5
            deposit, 2, 5, 500.012312
        ```
      expected result:

        ```
            client, available, held, total, locked
            1, 1.5, 0.0, 1.5, false
            2, 0.0, 0.0, 0.0, true
            
        ```

- [ ] When amounts have high level of decimals (testing the level of precision is output in requirement) and rounding:
  4 digits decimals, if 4thright most digit is 5, if 5th value to before is even keep 5, else make it 6

    - [ ] input:
        ```
            type, client, tx, amount
            deposit, 1, 1, 1.034534634
            deposit, 2, 2, 2.0313
            deposit, 1, 3, 2.053252345
            withdrawal, 1, 4, 1.54543
            deposit, 2, 5, 502.043612
            deposit, 3, 6, 1.42343234
            withdrawal, 3, 7, 0.3242352
            deposit, 4, 8, 1.42382234
            withdrawal, 4, 9, 0.3242352
        ```
      expected result:

        ```
            client, available, held, total, locked
            1, 1.5426, 0.0, 1.5426, false
            2, 502.0436, 0.0, 502.0436, false
            3, 1.0992, 0.0, 1.0992, false
            4, 1.0995, 0.0, 1.0995, false
        ```

- [ ] CSV Read performance
- [ ] Performance (only deposit and withdrawal 90% of records and 10% dispute that end in resolves -
  no chargeback to not allow freezing) correctness and speed measuring and peak memory and avg memory usage
  (simple measuring at the start of read and finish in main)
    - [ ] 10k txs
    - [ ] 100k txs
    - [ ] 1M txs
    - [ ] 50M txs
    - [ ] 100M txs
    - [ ] 500M txs
    - [ ] 1B txs
    - [ ] 2B txs


- [ ] Performance (all types) - random types correctness and speed measuring and peak memory and avg memory usage
  (simple measuring at the start of read and finish in main)
    - [ ] 10k txs
    - [ ] 100k txs
    - [ ] 1M txs
    - [ ] 50M txs
    - [ ] 100M txs
    - [ ] 500M txs
    - [ ] 1B txs
    - [ ] 2B txs

# Questions and Insights

- task::spawn(async move { async_read_csv(csv_path).await });
  means make sure async_read_csv starts running 

