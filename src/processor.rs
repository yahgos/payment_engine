use crate::{ClientAccount, Transaction, TransactionType};
use csv::{ReaderBuilder, Writer};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc::{Sender, channel};
use std::thread;

//Type aliases to simplify complex types and make clippy happy
type WorkerHandle = thread::JoinHandle<HashMap<u16, ClientState>>;
type WorkerPool = (Vec<WorkerHandle>, Vec<Sender<WorkerMessage>>);

/// Transaction record stored for dispute handling
#[derive(Debug, Clone)]
struct TransactionRecord {
    amount: f64,
    disputed: bool,
    is_deposit: bool, //track whether this was a deposit or withdrawal
}

/// State for a single client (account + transaction history)
#[derive(Debug)]
struct ClientState {
    account: ClientAccount,
    tx_history: HashMap<u32, TransactionRecord>,
}

impl ClientState {
    fn new(client_id: u16) -> Self {
        Self {
            account: ClientAccount::new(client_id),
            tx_history: HashMap::new(),
        }
    }
}

/// Message sent to worker threads
enum WorkerMessage {
    Transaction(Transaction),
    Shutdown,
}

/// Process CSV file with worker thread pool
/// Each client is consistently routed to the same worker thread
pub fn start_engine(path: &str) -> Result<(), Box<dyn Error>> {
    let num_workers = num_cpus::get();

    // Create worker threads and channels
    let (workers, senders) = create_worker_pool(num_workers);

    // Stream CSV and route transactions to workers
    route_transactions(path, &senders, num_workers)?;

    // Shutdown workers and collect results
    let all_states = shutdown_and_collect(workers, senders)?;

    // Write output
    write_output(&all_states)?;

    Ok(())
}

/// Create worker thread pool with one channel per worker
fn create_worker_pool(num_workers: usize) -> WorkerPool {
    let mut workers = Vec::with_capacity(num_workers);
    let mut senders = Vec::with_capacity(num_workers);

    for worker_id in 0..num_workers {
        let (tx, rx) = channel::<WorkerMessage>();
        senders.push(tx);

        let handle = thread::spawn(move || worker_thread(worker_id, rx));

        workers.push(handle);
    }

    (workers, senders)
}

/// Worker thread that processes transactions for assigned clients
fn worker_thread(
    worker_id: usize,
    receiver: std::sync::mpsc::Receiver<WorkerMessage>,
) -> HashMap<u16, ClientState> {
    let mut client_states: HashMap<u16, ClientState> = HashMap::new();

    // Process messages until shutdown
    while let Ok(message) = receiver.recv() {
        match message {
            WorkerMessage::Transaction(transaction) => {
                let client_id = transaction.client;

                // Get or create client state
                let state = client_states
                    .entry(client_id)
                    .or_insert_with(|| ClientState::new(client_id));

                // Process transaction
                process_single_transaction(state, transaction);
            }
            WorkerMessage::Shutdown => {
                break;
            }
        }
    }

    //this will provide log info without compromising stdout required format
    eprintln!(
        "Worker {} processed {} clients",
        worker_id,
        client_states.len()
    );
    client_states
}

/// Route transactions from CSV to appropriate worker threads
fn route_transactions(
    path: &str,
    senders: &[Sender<WorkerMessage>],
    num_workers: usize,
) -> Result<(), Box<dyn Error>> {
    let file = File::open(path)?;
    let buf_reader = BufReader::with_capacity(16 * 1024 * 1024, file);

    let mut csv_reader = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(buf_reader);

    // Stream transactions and route to workers
    for result in csv_reader.deserialize() {
        let transaction: Transaction = result?;

        // Route based on client ID - ensures same client always goes to same worker
        let worker_id = (transaction.client as usize) % num_workers;

        senders[worker_id]
            .send(WorkerMessage::Transaction(transaction))
            .map_err(|e| format!("Failed to send to worker: {}", e))?;
    }

    Ok(())
}

/// Shutdown workers and collect all client states
fn shutdown_and_collect(
    workers: Vec<thread::JoinHandle<HashMap<u16, ClientState>>>,
    senders: Vec<Sender<WorkerMessage>>,
) -> Result<HashMap<u16, ClientState>, Box<dyn Error>> {
    // Send shutdown signal to all workers
    for sender in senders {
        let _ = sender.send(WorkerMessage::Shutdown);
    }

    // Collect results from all workers
    let mut all_states = HashMap::new();

    for worker in workers {
        let worker_states = worker.join().map_err(|_| "Worker thread panicked")?;

        // Merge worker results
        all_states.extend(worker_states);
    }

    Ok(all_states)
}

fn process_single_transaction(state: &mut ClientState, transaction: Transaction) {
    if !transaction.is_valid() {
        return;
    }

    let account = &mut state.account;
    let tx_history = &mut state.tx_history;

    if account.locked && !transaction.is_dispute_action() {
        return;
    }

    match transaction.tx_type {
        TransactionType::Deposit => {
            if let Some(amount) = transaction.amount {
                account.available += amount;
                account.total += amount;

                tx_history.insert(
                    transaction.tx,
                    TransactionRecord {
                        amount,
                        disputed: false,
                        is_deposit: true, // Mark as deposit
                    },
                );
            }
        }

        TransactionType::Withdrawal => {
            if let Some(amount) = transaction.amount
                && account.available >= amount
            {
                account.available -= amount;
                account.total -= amount;

                tx_history.insert(
                    transaction.tx,
                    TransactionRecord {
                        amount,
                        disputed: false,
                        is_deposit: false, // Mark as withdrawal
                    },
                );
            }
        }

        TransactionType::Dispute => {
            if let Some(record) = tx_history.get_mut(&transaction.tx)
                && !record.disputed
            {
                if record.is_deposit {
                    // Disputing a deposit: hold the deposited funds
                    // available decreases, held increases, total unchanged
                    account.available -= record.amount;
                    account.held += record.amount;
                } else {
                    // Disputing a withdrawal: reverse the withdrawal but hold funds
                    // available unchanged, held increases, total increases
                    account.held += record.amount;
                    account.total += record.amount;
                }
                record.disputed = true;
            }
        }

        TransactionType::Resolve => {
            if let Some(record) = tx_history.get_mut(&transaction.tx)
                && record.disputed
            {
                if record.is_deposit {
                    // Resolving a deposit dispute: release held funds
                    // available increases, held decreases, total unchanged
                    account.available += record.amount;
                    account.held -= record.amount;
                } else {
                    // Resolving a withdrawal dispute: withdrawal was legitimate
                    // available unchanged, held decreases, total decreases
                    account.held -= record.amount;
                    account.total -= record.amount;
                }
                record.disputed = false;
            }
        }

        TransactionType::Chargeback => {
            if let Some(record) = tx_history.get(&transaction.tx)
                && record.disputed
            {
                if record.is_deposit {
                    // Chargeback on deposit: remove held funds
                    // held decreases, total decreases, lock account
                    account.held -= record.amount;
                    account.total -= record.amount;
                } else {
                    // Chargeback on withdrawal: withdrawal was fraudulent, return funds
                    // held decreases, available increases, total unchanged, lock account
                    account.held -= record.amount;
                    account.available += record.amount;
                }
                account.locked = true;
            }
        }
    }
}

/// Write results to stdout in CSV format
fn write_output(client_states: &HashMap<u16, ClientState>) -> Result<(), Box<dyn Error>> {
    let mut writer = Writer::from_writer(std::io::stdout());

    let mut client_ids: Vec<u16> = client_states.keys().copied().collect();
    client_ids.sort_unstable();

    for client_id in client_ids {
        if let Some(state) = client_states.get(&client_id) {
            writer.serialize(&state.account)?;
        }
    }

    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_processes_transactions() {
        let (tx, rx) = channel();

        // Send transactions
        tx.send(WorkerMessage::Transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(100.0),
        }))
        .unwrap();

        tx.send(WorkerMessage::Transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 2,
            amount: Some(50.0),
        }))
        .unwrap();

        tx.send(WorkerMessage::Shutdown).unwrap();

        let states = worker_thread(0, rx);

        assert_eq!(states.len(), 1);
        let state = states.get(&1).unwrap();
        assert_eq!(state.account.available, 150.0);
    }

    #[test]
    fn test_transaction_ordering() {
        let (tx, rx) = channel();

        // These must be processed in order
        tx.send(WorkerMessage::Transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(100.0),
        }))
        .unwrap();

        tx.send(WorkerMessage::Transaction(Transaction {
            tx_type: TransactionType::Withdrawal,
            client: 1,
            tx: 2,
            amount: Some(30.0),
        }))
        .unwrap();

        tx.send(WorkerMessage::Shutdown).unwrap();

        let states = worker_thread(0, rx);
        let state = states.get(&1).unwrap();

        assert_eq!(state.account.available, 70.0);
    }

    #[test]
    fn test_dispute_flow() {
        let (tx, rx) = channel();

        tx.send(WorkerMessage::Transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(100.0),
        }))
        .unwrap();

        tx.send(WorkerMessage::Transaction(Transaction {
            tx_type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: None,
        }))
        .unwrap();

        tx.send(WorkerMessage::Shutdown).unwrap();

        let states = worker_thread(0, rx);
        let state = states.get(&1).unwrap();

        assert_eq!(state.account.available, 0.0);
        assert_eq!(state.account.held, 100.0);
    }
}
