# Payments Engine

This application reads a series of financial transactions from a CSV file, processes them according to banking rules, and outputs the final account states. It supports basic operations (deposits and withdrawals) as well as dispute resolution mechanisms (disputes, resolves, and chargebacks).

The engine is designed to handle large transaction volumes efficiently through streaming I/O and parallel processing while maintaining correctness through deterministic transaction ordering per client.

## Features

- **Transaction Processing**: Deposits, withdrawals, disputes, resolves, and chargebacks
- **Account Management**: Tracks available funds, held funds, and account lock states
- **Dispute Resolution**: Full support for transaction disputes and reversals
- **Streaming Architecture**: Processes large files without loading entire dataset into memory
- **Parallel Processing**: Multi-threaded design for high throughput
- **Deterministic Execution**: Consistent results through client-based transaction routing

## Building and Running

### Build
```bash
cargo build --release
```

### Run
Example:
```bash
cargo run -- transactions.csv > accounts.csv
```

## Input Format

The input CSV must have the following columns: `type`, `client`, `tx`, `amount`

- **type**: Transaction type (deposit, withdrawal, dispute, resolve, chargeback)
- **client**: Client ID (u16)
- **tx**: Transaction ID (u32, globally unique)
- **amount**: Transaction amount (f64, up to 4 decimal places)

Example inputs can be found under tests/inputs

## Output Format

The output CSV contains the following columns: `client`, `available`, `held`, `total`, `locked`

- **client**: Client ID
- **available**: Funds available for withdrawal
- **held**: Funds held due to disputes
- **total**: Total funds (available + held)
- **locked**: Whether the account is locked due to chargeback

All monetary values are rounded to 4 decimal places.

## Project Structure
```
payments_engine/
|-- Cargo.toml
|-- README.md
|-- src/
|   |-- main.rs              # Entry point and CLI handling
|   |-- lib.rs               # Public API exports
|   |-- transaction.rs       # Transaction types and validation
|   |-- account.rs           # Client account state and serialization
|   |-- processor.rs         # Core transaction processing engine
|
|-- tests/
|   |-- integration_tests.rs # Integration tests
|   |-- inputs/              # Test input files
|   |-- expected_outputs/    # Expected output files for comparison
|   |-- actual_outputs/      # Generated outputs from tests
|   |-- test_all_scenarios.sh    # Automated test runner script
```

## Testing

The project includes comprehensive test coverage at multiple levels.

### Unit Tests

Unit tests are embedded within the source files and test individual components in isolation. Run with:
```bash
cargo test --lib
```

These tests verify:
- Transaction validation logic
- Worker thread message processing
- Individual transaction type handling (deposits, withdrawals, disputes)
- State management for client accounts

### Smoke Tests

Smoke tests validate edge cases and error handling without verifying output correctness. Run with:
```bash
cargo test smoke_tests
```

These tests cover:
- CSV parsing with whitespace
- Invalid transaction types and malformed data
- Boundary conditions (u16/u32 overflow)
- Empty files and edge cases
- Decimal precision parsing
- Large dataset processing (performance smoke test)

The smoke tests ensure the engine handles various input formats and error conditions gracefully without crashing.

### Scenario-Based Tests

Comprehensive end-to-end tests that validate complete transaction workflows and output correctness. Run with:
```bash
cargo test scenario_tests
```

Or specifically:
```bash
cargo test test_all_input_scenarios -- --nocapture
```

These tests:
1. Read CSV files from `tests/inputs/`
2. Process each file through the complete engine pipeline
3. Compare generated output against expected results in `tests/expected_outputs/`
4. Report detailed pass/fail status for each scenario
5. Write actual outputs to `tests/actual_outputs/` for inspection

Test scenarios cover:
- Basic deposits and withdrawals
- Dispute resolution flows (dispute → resolve, dispute → chargeback)
- Chargeback and account locking
- Multiple concurrent clients
- Edge cases (insufficient funds, invalid disputes, locked accounts)
- Precision handling (4 decimal places)
- Complex multi-step transaction sequences

### Running All Tests

Execute the complete test suite:
```bash
cargo test
```

This runs all unit tests, smoke tests, and scenario-based tests in sequence.

For verbose output showing test progress:
```bash
cargo test -- --nocapture
```

### Test Coverage

The combined test suite validates:
- Correct transaction processing order
- Dispute state transitions and idempotency
- Account locking behavior after chargebacks
- Insufficient funds handling
- Invalid transaction reference handling
- Multi-client parallel processing correctness
- CSV parsing with various whitespace formats
- Numerical precision (4 decimal places)
- Deterministic output for identical inputs
- Error handling for malformed input

## Design and Architecture

### Multi-threaded Processing

The engine uses a worker pool architecture to process transactions in parallel while maintaining correctness.
The number of workers is automatically set to match the number of CPU cores available on the system.

### Transaction Routing

To avoid race conditions while maintaining parallelism, transactions are routed to workers based on client ID:
```rust
worker_id = client_id % num_workers
```

This ensures:
- All transactions for a given client are processed by the same worker thread
- Transactions for the same client are processed sequentially in file order
- Different clients can be processed in parallel without contention
- No locks or synchronization primitives are needed for transaction processing

### Memory Efficiency

The engine streams data rather than loading entire files into memory:
- CSV is read in chunks using a buffered reader (16MB buffer)
- Transactions are processed one at a time and immediately discarded
- Only client account states and transaction history are retained
- Memory usage scales with the number of unique clients and transactions, not file size

## Assumptions

The implementation makes the following assumptions consistent with banking transaction processors:

1. Each client has a single asset account
2. Transaction IDs are globally unique (not per-client)
3. Transactions appear in chronological order in the input file
4. Dispute-related operations (dispute, resolve, chargeback) can only reference deposit transactions
5. Once an account is locked via chargeback, it remains permanently locked
6. Withdrawals that would result in negative balance are rejected
7. Disputes on non-existent transactions are treated as errors and ignored
8. Multiple disputes on the same transaction are idempotent (subsequent disputes ignored)
9. Disputes on withdrawals hold the disputed amount until resolution.
